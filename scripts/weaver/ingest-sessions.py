#!/usr/bin/env python3
"""
Ingest Claude Code session JSONL files into ECC causal graph format.

Each session becomes a conversation in the graph:
- Human messages (type=user, string content) -> goal/direction nodes
- Assistant messages (type=assistant) -> action/implementation nodes
- Tool calls (tool_use blocks in assistant content) -> mutation nodes
- Tool results (type=user, tool_result blocks) -> observation nodes
- Agent spawns -> sub-conversation root nodes

Edges:
- Follows: sequential messages in the conversation
- Causes: human message -> assistant response
- Enables: tool call -> file change -> subsequent tool call
- TriggeredBy: agent spawn triggered by a message
- EvidenceFor: tool result confirms/denies a hypothesis

JSONL format (Claude Code):
  Each line has: type, uuid, parentUuid, timestamp, sessionId, message?, ...
  type in {user, assistant, system, progress, file-history-snapshot, queue-operation}
  message.content is either a string (human text) or list of blocks:
    {type: "text", text: "..."} | {type: "tool_use", name, input} |
    {type: "tool_result", tool_use_id, content} | {type: "thinking", thinking}
"""

import json
import sys
import os
from pathlib import Path
from datetime import datetime
from collections import defaultdict


def extract_text_content(message):
    """Pull human-readable text from a message dict."""
    if not isinstance(message, dict):
        return ''
    content = message.get('content', '')
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        texts = []
        for block in content:
            if isinstance(block, dict):
                if block.get('type') == 'text':
                    texts.append(block.get('text', ''))
                elif block.get('type') == 'thinking':
                    # Include thinking as context but tag it
                    pass  # skip thinking for content preview
            elif isinstance(block, str):
                texts.append(block)
        return '\n'.join(texts)
    return ''


def extract_tool_uses(message):
    """Extract tool_use blocks from an assistant message."""
    tools = []
    if not isinstance(message, dict):
        return tools
    content = message.get('content', [])
    if not isinstance(content, list):
        return tools
    for block in content:
        if isinstance(block, dict) and block.get('type') == 'tool_use':
            tools.append({
                'id': block.get('id', ''),
                'name': block.get('name', ''),
                'input': block.get('input', {}),
            })
    return tools


def extract_tool_results(message):
    """Extract tool_result blocks from a user message."""
    results = []
    if not isinstance(message, dict):
        return results
    content = message.get('content', [])
    if not isinstance(content, list):
        return results
    for block in content:
        if isinstance(block, dict) and block.get('type') == 'tool_result':
            results.append({
                'tool_use_id': block.get('tool_use_id', ''),
                'is_error': block.get('is_error', False),
            })
    return results


def has_thinking(message):
    """Check if assistant message has extended thinking."""
    if not isinstance(message, dict):
        return False
    content = message.get('content', [])
    if not isinstance(content, list):
        return False
    return any(
        isinstance(b, dict) and b.get('type') == 'thinking'
        for b in content
    )


def classify_tool(name):
    """Classify a tool call by category."""
    if name in ('Read', 'Glob', 'Grep', 'LS', 'Skill', 'ToolSearch',
                'WebSearch', 'WebFetch'):
        return 'read'
    elif name in ('Write', 'Edit', 'MultiEdit', 'NotebookEdit'):
        return 'write'
    elif name == 'Bash':
        return 'execute'
    elif name in ('Agent', 'Task'):
        return 'spawn'
    elif name in ('TodoRead', 'TodoWrite'):
        return 'plan'
    return 'other'


def classify_message(content):
    """Tag a message based on content keywords."""
    tags = []
    lower = content.lower()[:2000]  # cap scan length

    tag_rules = [
        (['k6', 'mesh', 'transport', 'noise', 'quic'], 'mesh-networking'),
        (['ecc', 'causal', 'cognitive', 'weaver', 'hnsw'], 'ecc'),
        (['test', 'assert', 'cargo test', '#[test]'], 'testing'),
        (['governance', 'gate', 'effect vector'], 'governance'),
        (['chain', 'exochain', 'hash', 'signing'], 'exochain'),
        (['wasm', 'sandbox', 'fuel', 'wasmtime'], 'wasm'),
        (['agent', 'spawn', 'supervisor', 'swarm'], 'agents'),
        (['doc', 'fumadoc', 'readme', 'mdx'], 'documentation'),
        (['commit', 'push', 'merge', 'pr '], 'git-ops'),
        (['sparc', 'plan', 'sprint', 'symposium'], 'planning'),
        (['gap', 'missing', 'todo', 'remaining'], 'gap-analysis'),
        (['build', 'cargo', 'compile', 'clippy'], 'build'),
        (['kernel', 'weftos', 'claw'], 'kernel'),
        (['error', 'bug', 'fix', 'panic', 'fail'], 'debugging'),
    ]

    for keywords, tag in tag_rules:
        if any(w in lower for w in keywords):
            tags.append(tag)

    return tags if tags else ['general']


def tool_input_preview(tool_input):
    """Create a brief preview of tool input."""
    if not isinstance(tool_input, dict):
        return str(tool_input)[:150]
    # For common tools, extract the key field
    if 'file_path' in tool_input:
        return tool_input['file_path']
    if 'command' in tool_input:
        cmd = tool_input['command']
        return cmd[:200] if isinstance(cmd, str) else str(cmd)[:200]
    if 'pattern' in tool_input:
        return f"pattern={tool_input['pattern']}"
    if 'query' in tool_input:
        return f"query={tool_input['query']}"
    return json.dumps(tool_input)[:150]


def make_edge(from_id, to_id, edge_type, weight=1.0):
    return {'from': from_id, 'to': to_id, 'type': edge_type, 'weight': weight}


def parse_session(filepath):
    """Parse a single JSONL session file into nodes and edges."""
    nodes = []
    edges = []
    prev_node_id = None
    session_id = Path(filepath).stem
    sid = session_id[:12]  # shortened for node IDs

    # Detect if this is a subagent file
    is_subagent = session_id.startswith('agent-')
    agent_id = None
    if is_subagent:
        agent_id = session_id.replace('agent-', '')

    line_count = 0
    error_count = 0

    with open(filepath, 'r', errors='replace') as f:
        for line_num, line in enumerate(f):
            line = line.strip()
            if not line:
                continue
            try:
                entry = json.loads(line)
            except json.JSONDecodeError:
                error_count += 1
                continue

            line_count += 1
            entry_type = entry.get('type', '')
            timestamp = entry.get('timestamp', '')
            uuid = entry.get('uuid', '')
            message = entry.get('message', {})

            # --- Human / User messages ---
            if entry_type == 'user':
                # Check if it's actual human text or tool results
                tool_results = extract_tool_results(message)
                if tool_results:
                    # This is a tool result return -- create observation nodes
                    for tr in tool_results:
                        node_id = f'obs:{sid}:{line_num}'
                        node = {
                            'id': node_id,
                            'type': 'tool_result',
                            'session': session_id,
                            'line': line_num,
                            'tool_use_id': tr['tool_use_id'],
                            'is_error': tr['is_error'],
                            'timestamp': timestamp,
                            'tags': ['error-result'] if tr['is_error'] else ['ok-result'],
                        }
                        if agent_id:
                            node['agent_id'] = agent_id
                        nodes.append(node)
                        if prev_node_id:
                            edges.append(make_edge(prev_node_id, node_id, 'EvidenceFor'))
                        prev_node_id = node_id
                else:
                    # Human text input
                    text = extract_text_content(message)
                    if not text or len(text.strip()) < 2:
                        continue
                    node_id = f'hum:{sid}:{line_num}'
                    node = {
                        'id': node_id,
                        'type': 'human_message',
                        'session': session_id,
                        'line': line_num,
                        'content_preview': text[:300],
                        'content_length': len(text),
                        'timestamp': timestamp,
                        'tags': classify_message(text),
                    }
                    if agent_id:
                        node['agent_id'] = agent_id
                    nodes.append(node)
                    if prev_node_id:
                        edges.append(make_edge(prev_node_id, node_id, 'Follows'))
                    prev_node_id = node_id

            # --- Assistant messages ---
            elif entry_type == 'assistant':
                text = extract_text_content(message)
                tool_uses = extract_tool_uses(message)
                thinking = has_thinking(message)

                # Create assistant node only if there's text content
                if text and len(text.strip()) >= 2:
                    node_id = f'ast:{sid}:{line_num}'
                    spawns = any(t['name'] in ('Agent', 'Task') for t in tool_uses)
                    node = {
                        'id': node_id,
                        'type': 'assistant_message',
                        'session': session_id,
                        'line': line_num,
                        'content_preview': text[:300],
                        'content_length': len(text),
                        'timestamp': timestamp,
                        'tags': classify_message(text),
                        'has_thinking': thinking,
                        'spawns_agents': spawns,
                        'tool_count': len(tool_uses),
                    }
                    if agent_id:
                        node['agent_id'] = agent_id
                    nodes.append(node)
                    if prev_node_id:
                        edges.append(make_edge(prev_node_id, node_id, 'Causes'))
                    prev_node_id = node_id
                elif not tool_uses:
                    # Assistant message with only thinking, no text or tools
                    continue

                # Create tool call nodes
                for i, tu in enumerate(tool_uses):
                    tool_id = f'tool:{sid}:{line_num}:{i}'
                    ttype = classify_tool(tu['name'])
                    node = {
                        'id': tool_id,
                        'type': 'tool_call',
                        'session': session_id,
                        'line': line_num,
                        'tool_name': tu['name'],
                        'tool_type': ttype,
                        'tool_use_id': tu['id'],
                        'input_preview': tool_input_preview(tu['input']),
                        'timestamp': timestamp,
                    }
                    if agent_id:
                        node['agent_id'] = agent_id
                    nodes.append(node)

                    # Edge from assistant text to tool call, or from prev node
                    parent = f'ast:{sid}:{line_num}' if text and len(text.strip()) >= 2 else prev_node_id
                    if parent:
                        etype = 'TriggeredBy' if ttype == 'spawn' else 'Enables'
                        edges.append(make_edge(parent, tool_id, etype))

                    prev_node_id = tool_id

            # --- System messages (hooks, stop reasons, etc.) ---
            elif entry_type == 'system':
                subtype = entry.get('subtype', '')
                if subtype:
                    node_id = f'sys:{sid}:{line_num}'
                    node = {
                        'id': node_id,
                        'type': 'system_event',
                        'session': session_id,
                        'line': line_num,
                        'subtype': subtype,
                        'timestamp': timestamp,
                        'tags': ['system'],
                    }
                    nodes.append(node)
                    if prev_node_id:
                        edges.append(make_edge(prev_node_id, node_id, 'Follows'))
                    prev_node_id = node_id

            # Skip: file-history-snapshot, progress, queue-operation
            # These don't contribute causal nodes

    return nodes, edges, line_count, error_count


def process_directory(session_dir, label):
    """Process all JSONL files in a directory."""
    all_nodes = []
    all_edges = []
    total_lines = 0
    total_errors = 0

    jsonl_files = sorted(Path(session_dir).glob('*.jsonl'))
    for f in jsonl_files:
        try:
            nodes, edges, lines, errors = parse_session(str(f))
            all_nodes.extend(nodes)
            all_edges.extend(edges)
            total_lines += lines
            total_errors += errors
        except Exception as e:
            print(f'  Warning: {f.name}: {e}', file=sys.stderr)

    return all_nodes, all_edges, total_lines, total_errors, len(jsonl_files)


def main():
    base = Path('.weftos/sessions')
    if not base.exists():
        print(f'Error: {base} does not exist', file=sys.stderr)
        sys.exit(1)

    all_nodes = []
    all_edges = []
    stats = {}

    # Process each directory
    dirs_to_scan = [
        ('current', 'current'),
        ('history/clawft', 'clawft-history'),
        ('history/moltworker', 'moltworker-history'),
        ('history/barni', 'barni-history'),
        ('subagents', 'subagents'),
        ('predecessors/weave-nn', 'weave-nn'),
        ('predecessors/weave-nn-kg-agent', 'weave-nn-kg'),
    ]

    for subdir, label in dirs_to_scan:
        dir_path = base / subdir
        if not dir_path.exists():
            continue
        jsonl_count = len(list(dir_path.glob('*.jsonl')))
        if jsonl_count == 0:
            continue

        print(f'Processing {label} ({jsonl_count} files)...')
        nodes, edges, lines, errors, fcount = process_directory(dir_path, label)

        # Tag all nodes with their source
        for n in nodes:
            n['source'] = label

        all_nodes.extend(nodes)
        all_edges.extend(edges)
        stats[label] = {
            'files': fcount,
            'lines_parsed': lines,
            'parse_errors': errors,
            'nodes': len(nodes),
            'edges': len(edges),
        }
        print(f'  {fcount} files, {lines} lines -> {len(nodes)} nodes, {len(edges)} edges'
              + (f' ({errors} parse errors)' if errors else ''))

    if not all_nodes:
        print('No nodes extracted.', file=sys.stderr)
        sys.exit(1)

    # --- Cross-session edges ---
    # Link subagent sessions back to their parent session
    # Subagent files contain a sessionId pointing to the parent
    agent_roots = {}
    for n in all_nodes:
        if n.get('source') == 'subagents' and n.get('type') == 'human_message':
            aid = n.get('agent_id', '')
            if aid and aid not in agent_roots:
                agent_roots[aid] = n['id']

    # Link agent spawn tool calls to the subagent root
    for n in all_nodes:
        if n.get('type') == 'tool_call' and n.get('tool_name') in ('Agent', 'Task'):
            # Try to match by input preview containing agent ID
            preview = n.get('input_preview', '')
            for aid, root_id in agent_roots.items():
                if aid in preview:
                    all_edges.append(make_edge(n['id'], root_id, 'TriggeredBy', 0.8))

    # --- Statistics ---
    tag_counts = defaultdict(int)
    for n in all_nodes:
        for tag in n.get('tags', []):
            tag_counts[tag] += 1

    tool_counts = defaultdict(int)
    tool_type_counts = defaultdict(int)
    for n in all_nodes:
        if n['type'] == 'tool_call':
            tool_counts[n.get('tool_name', 'unknown')] += 1
            tool_type_counts[n.get('tool_type', 'unknown')] += 1

    node_type_counts = defaultdict(int)
    for n in all_nodes:
        node_type_counts[n['type']] += 1

    edge_type_counts = defaultdict(int)
    for e in all_edges:
        edge_type_counts[e['type']] += 1

    # Sessions list
    sessions_seen = set()
    for n in all_nodes:
        sessions_seen.add(n.get('session', ''))

    # --- Build output ---
    output = {
        'domain': 'clawft-weftos',
        'source': 'session-logs',
        'generated_at': datetime.utcnow().isoformat() + 'Z',
        'nodes': all_nodes,
        'edges': all_edges,
        'stats': {
            'total_nodes': len(all_nodes),
            'total_edges': len(all_edges),
            'total_sessions': len(sessions_seen),
            'sources': stats,
            'node_types': dict(sorted(node_type_counts.items(), key=lambda x: -x[1])),
            'edge_types': dict(sorted(edge_type_counts.items(), key=lambda x: -x[1])),
            'tag_distribution': dict(sorted(tag_counts.items(), key=lambda x: -x[1])),
            'tool_distribution': dict(sorted(tool_counts.items(), key=lambda x: -x[1])),
            'tool_type_distribution': dict(sorted(tool_type_counts.items(), key=lambda x: -x[1])),
        },
    }

    # Write output
    out_path = Path('.weftos/graph/session-logs.json')
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, 'w') as f:
        json.dump(output, f, indent=2, default=str)

    # --- Summary ---
    print(f'\n{"="*60}')
    print(f'Total: {len(all_nodes)} nodes, {len(all_edges)} edges, {len(sessions_seen)} sessions')
    print(f'Written to {out_path}')

    print(f'\nNode types:')
    for nt, count in sorted(node_type_counts.items(), key=lambda x: -x[1]):
        print(f'  {nt}: {count}')

    print(f'\nEdge types:')
    for et, count in sorted(edge_type_counts.items(), key=lambda x: -x[1]):
        print(f'  {et}: {count}')

    print(f'\nTag distribution (top 15):')
    for tag, count in sorted(tag_counts.items(), key=lambda x: -x[1])[:15]:
        print(f'  {tag}: {count}')

    print(f'\nTool distribution (top 15):')
    for tool, count in sorted(tool_counts.items(), key=lambda x: -x[1])[:15]:
        print(f'  {tool}: {count}')


if __name__ == '__main__':
    main()
