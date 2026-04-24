"use strict";
/**
 * Session Management for multi-turn conversations
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.SessionManager = void 0;
/**
 * Session Manager for multi-turn conversations
 *
 * @example
 * ```typescript
 * import { RuvLLM, SessionManager } from '@ruvector/ruvllm';
 *
 * const llm = new RuvLLM();
 * const sessions = new SessionManager(llm);
 *
 * // Create a new session
 * const session = sessions.create();
 *
 * // Chat with context
 * const response1 = sessions.chat(session.id, 'What is Python?');
 * const response2 = sessions.chat(session.id, 'How do I install it?');
 * // Second query automatically has context from first
 * ```
 */
class SessionManager {
    constructor(llm) {
        this.sessions = new Map();
        this.llm = llm;
    }
    /**
     * Create a new conversation session
     */
    create(metadata) {
        const id = `session-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
        const session = {
            id,
            createdAt: new Date(),
            messageCount: 0,
            messages: [],
            context: [],
            activeMemoryIds: [],
            metadata: metadata ?? {},
        };
        this.sessions.set(id, session);
        return session;
    }
    /**
     * Get session by ID
     */
    get(sessionId) {
        return this.sessions.get(sessionId);
    }
    /**
     * Chat within a session (maintains context)
     */
    chat(sessionId, message, config) {
        const session = this.sessions.get(sessionId);
        if (!session) {
            throw new Error(`Session not found: ${sessionId}`);
        }
        // Add user message
        session.messages.push({
            role: 'user',
            content: message,
            timestamp: new Date(),
        });
        // Build context from recent messages
        const contextWindow = this.buildContext(session);
        // Query with context
        const prompt = contextWindow ? `${contextWindow}\n\nUser: ${message}` : message;
        const response = this.llm.query(prompt, config);
        // Add assistant response
        session.messages.push({
            role: 'assistant',
            content: response.text,
            timestamp: new Date(),
            requestId: response.requestId,
        });
        session.messageCount = session.messages.length;
        return response;
    }
    /**
     * Add system message to session
     */
    addSystemMessage(sessionId, content) {
        const session = this.sessions.get(sessionId);
        if (!session) {
            throw new Error(`Session not found: ${sessionId}`);
        }
        session.messages.push({
            role: 'system',
            content,
            timestamp: new Date(),
        });
        session.messageCount = session.messages.length;
    }
    /**
     * Add context to session (persisted to memory)
     */
    addContext(sessionId, context) {
        const session = this.sessions.get(sessionId);
        if (!session) {
            throw new Error(`Session not found: ${sessionId}`);
        }
        session.context.push(context);
        // Also store in memory for retrieval
        const memoryId = this.llm.addMemory(context, {
            sessionId,
            type: 'context',
            timestamp: new Date().toISOString(),
        });
        session.activeMemoryIds.push(memoryId);
        return memoryId;
    }
    /**
     * Get conversation history
     */
    getHistory(sessionId, limit) {
        const session = this.sessions.get(sessionId);
        if (!session) {
            return [];
        }
        const messages = session.messages;
        return limit ? messages.slice(-limit) : messages;
    }
    /**
     * Clear session history (keep session active)
     */
    clearHistory(sessionId) {
        const session = this.sessions.get(sessionId);
        if (session) {
            session.messages = [];
            session.context = [];
            session.messageCount = 0;
        }
    }
    /**
     * End and delete session
     */
    end(sessionId) {
        return this.sessions.delete(sessionId);
    }
    /**
     * List all active sessions
     */
    list() {
        return Array.from(this.sessions.values());
    }
    /**
     * Export session as JSON
     */
    export(sessionId) {
        const session = this.sessions.get(sessionId);
        if (!session) {
            return null;
        }
        return JSON.stringify(session, null, 2);
    }
    /**
     * Import session from JSON
     */
    import(json) {
        const data = JSON.parse(json);
        const session = {
            ...data,
            createdAt: new Date(data.createdAt),
            messages: data.messages.map((m) => ({
                ...m,
                timestamp: new Date(m.timestamp),
            })),
        };
        this.sessions.set(session.id, session);
        return session;
    }
    /**
     * Build context string from recent messages
     */
    buildContext(session, maxMessages = 10) {
        const recent = session.messages.slice(-maxMessages);
        if (recent.length === 0) {
            return '';
        }
        const contextParts = [];
        // Add persistent context
        if (session.context.length > 0) {
            contextParts.push('Context:\n' + session.context.join('\n'));
        }
        // Add conversation history
        const history = recent
            .map(m => {
            const role = m.role === 'user' ? 'User' : m.role === 'assistant' ? 'Assistant' : 'System';
            return `${role}: ${m.content}`;
        })
            .join('\n');
        if (history) {
            contextParts.push('Conversation:\n' + history);
        }
        return contextParts.join('\n\n');
    }
}
exports.SessionManager = SessionManager;
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoic2Vzc2lvbi5qcyIsInNvdXJjZVJvb3QiOiIiLCJzb3VyY2VzIjpbIi4uLy4uL3NyYy9zZXNzaW9uLnRzIl0sIm5hbWVzIjpbXSwibWFwcGluZ3MiOiI7QUFBQTs7R0FFRzs7O0FBU0g7Ozs7Ozs7Ozs7Ozs7Ozs7OztHQWtCRztBQUNILE1BQWEsY0FBYztJQUl6QixZQUFZLEdBQXNKO1FBSDFKLGFBQVEsR0FBcUMsSUFBSSxHQUFHLEVBQUUsQ0FBQztRQUk3RCxJQUFJLENBQUMsR0FBRyxHQUFHLEdBQUcsQ0FBQztJQUNqQixDQUFDO0lBRUQ7O09BRUc7SUFDSCxNQUFNLENBQUMsUUFBa0M7UUFDdkMsTUFBTSxFQUFFLEdBQUcsV0FBVyxJQUFJLENBQUMsR0FBRyxFQUFFLElBQUksSUFBSSxDQUFDLE1BQU0sRUFBRSxDQUFDLFFBQVEsQ0FBQyxFQUFFLENBQUMsQ0FBQyxLQUFLLENBQUMsQ0FBQyxFQUFFLENBQUMsQ0FBQyxFQUFFLENBQUM7UUFDN0UsTUFBTSxPQUFPLEdBQXdCO1lBQ25DLEVBQUU7WUFDRixTQUFTLEVBQUUsSUFBSSxJQUFJLEVBQUU7WUFDckIsWUFBWSxFQUFFLENBQUM7WUFDZixRQUFRLEVBQUUsRUFBRTtZQUNaLE9BQU8sRUFBRSxFQUFFO1lBQ1gsZUFBZSxFQUFFLEVBQUU7WUFDbkIsUUFBUSxFQUFFLFFBQVEsSUFBSSxFQUFFO1NBQ3pCLENBQUM7UUFDRixJQUFJLENBQUMsUUFBUSxDQUFDLEdBQUcsQ0FBQyxFQUFFLEVBQUUsT0FBTyxDQUFDLENBQUM7UUFDL0IsT0FBTyxPQUFPLENBQUM7SUFDakIsQ0FBQztJQUVEOztPQUVHO0lBQ0gsR0FBRyxDQUFDLFNBQWlCO1FBQ25CLE9BQU8sSUFBSSxDQUFDLFFBQVEsQ0FBQyxHQUFHLENBQUMsU0FBUyxDQUFDLENBQUM7SUFDdEMsQ0FBQztJQUVEOztPQUVHO0lBQ0gsSUFBSSxDQUFDLFNBQWlCLEVBQUUsT0FBZSxFQUFFLE1BQXlCO1FBQ2hFLE1BQU0sT0FBTyxHQUFHLElBQUksQ0FBQyxR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QyxJQUFJLEtBQUssTUFBTSxDQUFDLENBQUMsQ0FBQyxNQUFNLENBQUMsQ0FBQyxDQUFDLENBQUMsQ0FBQyxJQUFJLEtBQUssV0FBVyxDQUFDLENBQUMsQ0FBQyxXQUFXLENBQUMsQ0FBQyxDQUFDLFFBQVEsQ0FBQztZQUMxRixPQUFPLEdBQUcsSUFBSSxLQUFLLENBQUMsQ0FBQyxPQUFPLEVBQUUsQ0FBQztRQUNqQyxDQUFDLENBQUM7YUFDRCxJQUFJLENBQUMsSUFBSSxDQUFDLENBQUM7UUFFZCxJQUFJLE9BQU8sRUFBRSxDQUFDO1lBQ1osWUFBWSxDQUFDLElBQUksQ0FBQyxpQkFBaUIsR0FBRyxPQUFPLENBQUMsQ0FBQztRQUNqRCxDQUFDO1FBRUQsT0FBTyxZQUFZLENBQUMsSUFBSSxDQUFDLE1BQU0sQ0FBQyxDQUFDO0lBQ25DLENBQUM7Q0FDRjtBQS9NRCx3Q0ErTUMiLCJzb3VyY2VzQ29udGVudCI6WyIvKipcbiAqIFNlc3Npb24gTWFuYWdlbWVudCBmb3IgbXVsdGktdHVybiBjb252ZXJzYXRpb25zXG4gKi9cblxuaW1wb3J0IHtcbiAgQ29udmVyc2F0aW9uU2Vzc2lvbixcbiAgQ29udmVyc2F0aW9uTWVzc2FnZSxcbiAgUXVlcnlSZXNwb25zZSxcbiAgR2VuZXJhdGlvbkNvbmZpZyxcbn0gZnJvbSAnLi90eXBlcyc7XG5cbi8qKlxuICogU2Vzc2lvbiBNYW5hZ2VyIGZvciBtdWx0aS10dXJuIGNvbnZlcnNhdGlvbnNcbiAqXG4gKiBAZXhhbXBsZVxuICogYGBgdHlwZXNjcmlwdFxuICogaW1wb3J0IHsgUnV2TExNLCBTZXNzaW9uTWFuYWdlciB9IGZyb20gJ0BydXZlY3Rvci9ydXZsbG0nO1xuICpcbiAqIGNvbnN0IGxsbSA9IG5ldyBSdXZMTE0oKTtcbiAqIGNvbnN0IHNlc3Npb25zID0gbmV3IFNlc3Npb25NYW5hZ2VyKGxsbSk7XG4gKlxuICogLy8gQ3JlYXRlIGEgbmV3IHNlc3Npb25cbiAqIGNvbnN0IHNlc3Npb24gPSBzZXNzaW9ucy5jcmVhdGUoKTtcbiAqXG4gKiAvLyBDaGF0IHdpdGggY29udGV4dFxuICogY29uc3QgcmVzcG9uc2UxID0gc2Vzc2lvbnMuY2hhdChzZXNzaW9uLmlkLCAnV2hhdCBpcyBQeXRob24/Jyk7XG4gKiBjb25zdCByZXNwb25zZTIgPSBzZXNzaW9ucy5jaGF0KHNlc3Npb24uaWQsICdIb3cgZG8gSSBpbnN0YWxsIGl0PycpO1xuICogLy8gU2Vjb25kIHF1ZXJ5IGF1dG9tYXRpY2FsbHkgaGFzIGNvbnRleHQgZnJvbSBmaXJzdFxuICogYGBgXG4gKi9cbmV4cG9ydCBjbGFzcyBTZXNzaW9uTWFuYWdlciB7XG4gIHByaXZhdGUgc2Vzc2lvbnM6IE1hcDxzdHJpbmcsIENvbnZlcnNhdGlvblNlc3Npb24+ID0gbmV3IE1hcCgpO1xuICBwcml2YXRlIGxsbTogeyBxdWVyeTogKHRleHQ6IHN0cmluZywgY29uZmlnPzogR2VuZXJhdGlvbkNvbmZpZykgPT4gUXVlcnlSZXNwb25zZTsgYWRkTWVtb3J5OiAoY29udGVudDogc3RyaW5nLCBtZXRhZGF0YT86IFJlY29yZDxzdHJpbmcsIHVua25vd24+KSA9PiBudW1iZXIgfTtcblxuICBjb25zdHJ1Y3RvcihsbG06IHsgcXVlcnk6ICh0ZXh0OiBzdHJpbmcsIGNvbmZpZz86IEdlbmVyYXRpb25Db25maWcpID0+IFF1ZXJ5UmVzcG9uc2U7IGFkZE1lbW9yeTogKGNvbnRlbnQ6IHN0cmluZywgbWV0YWRhdGE/OiBSZWNvcmQ8c3RyaW5nLCB1bmtub3duPikgPT4gbnVtYmVyIH0pIHtcbiAgICB0aGlzLmxsbSA9IGxsbTtcbiAgfVxuXG4gIC8qKlxuICAgKiBDcmVhdGUgYSBuZXcgY29udmVyc2F0aW9uIHNlc3Npb25cbiAgICovXG4gIGNyZWF0ZShtZXRhZGF0YT86IFJlY29yZDxzdHJpbmcsIHVua25vd24+KTogQ29udmVyc2F0aW9uU2Vzc2lvbiB7XG4gICAgY29uc3QgaWQgPSBgc2Vzc2lvbi0ke0RhdGUubm93KCl9LSR7TWF0aC5yYW5kb20oKS50b1N0cmluZygzNikuc2xpY2UoMiwgOCl9YDtcbiAgICBjb25zdCBzZXNzaW9uOiBDb252ZXJzYXRpb25TZXNzaW9uID0ge1xuICAgICAgaWQsXG4gICAgICBjcmVhdGVkQXQ6IG5ldyBEYXRlKCksXG4gICAgICBtZXNzYWdlQ291bnQ6IDAsXG4gICAgICBtZXNzYWdlczogW10sXG4gICAgICBjb250ZXh0OiBbXSxcbiAgICAgIGFjdGl2ZU1lbW9yeUlkczogW10sXG4gICAgICBtZXRhZGF0YTogbWV0YWRhdGEgPz8ge30sXG4gICAgfTtcbiAgICB0aGlzLnNlc3Npb25zLnNldChpZCwgc2Vzc2lvbik7XG4gICAgcmV0dXJuIHNlc3Npb247XG4gIH1cblxuICAvKipcbiAgICogR2V0IHNlc3Npb24gYnkgSURcbiAgICovXG4gIGdldChzZXNzaW9uSWQ6IHN0cmluZyk6IENvbnZlcnNhdGlvblNlc3Npb24gfCB1bmRlZmluZWQge1xuICAgIHJldHVybiB0aGlzLnNlc3Npb25zLmdldChzZXNzaW9uSWQpO1xuICB9XG5cbiAgLyoqXG4gICAqIENoYXQgd2l0aGluIGEgc2Vzc2lvbiAobWFpbnRhaW5zIGNvbnRleHQpXG4gICAqL1xuICBjaGF0KHNlc3Npb25JZDogc3RyaW5nLCBtZXNzYWdlOiBzdHJpbmcsIGNvbmZpZz86IEdlbmVyYXRpb25Db25maWcpOiBRdWVyeVJlc3BvbnNlIHtcbiAgICBjb25zdCBzZXNzaW9uID0gdGhpcy5zZXNzaW9ucy5nZXQoc2Vzc2lvbklkKTtcbiAgICBpZiAoIXNlc3Npb24pIHtcbiAgICAgIHRocm93IG5ldyBFcnJvcihgU2Vzc2lvbiBub3QgZm91bmQ6ICR7c2Vzc2lvbklkfWApO1xuICAgIH1cblxuICAgIC8vIEFkZCB1c2VyIG1lc3NhZ2VcbiAgICBzZXNzaW9uLm1lc3NhZ2VzLnB1c2goe1xuICAgICAgcm9sZTogJ3VzZXInLFxuICAgICAgY29udGVudDogbWVzc2FnZSxcbiAgICAgIHRpbWVzdGFtcDogbmV3IERhdGUoKSxcbiAgICB9KTtcblxuICAgIC8vIEJ1aWxkIGNvbnRleHQgZnJvbSByZWNlbnQgbWVzc2FnZXNcbiAgICBjb25zdCBjb250ZXh0V2luZG93ID0gdGhpcy5idWlsZENvbnRleHQoc2Vzc2lvbik7XG5cbiAgICAvLyBRdWVyeSB3aXRoIGNvbnRleHRcbiAgICBjb25zdCBwcm9tcHQgPSBjb250ZXh0V2luZG93ID8gYCR7Y29udGV4dFdpbmRvd31cXG5cXG5Vc2VyOiAke21lc3NhZ2V9YCA6IG1lc3NhZ2U7XG4gICAgY29uc3QgcmVzcG9uc2UgPSB0aGlzLmxsbS5xdWVyeShwcm9tcHQsIGNvbmZpZyk7XG5cbiAgICAvLyBBZGQgYXNzaXN0YW50IHJlc3BvbnNlXG4gICAgc2Vzc2lvbi5tZXNzYWdlcy5wdXNoKHtcbiAgICAgIHJvbGU6ICdhc3Npc3RhbnQnLFxuICAgICAgY29udGVudDogcmVzcG9uc2UudGV4dCxcbiAgICAgIHRpbWVzdGFtcDogbmV3IERhdGUoKSxcbiAgICAgIHJlcXVlc3RJZDogcmVzcG9uc2UucmVxdWVzdElkLFxuICAgIH0pO1xuXG4gICAgc2Vzc2lvbi5tZXNzYWdlQ291bnQgPSBzZXNzaW9uLm1lc3NhZ2VzLmxlbmd0aDtcblxuICAgIHJldHVybiByZXNwb25zZTtcbiAgfVxuXG4gIC8qKlxuICAgKiBBZGQgc3lzdGVtIG1lc3NhZ2UgdG8gc2Vzc2lvblxuICAgKi9cbiAgYWRkU3lzdGVtTWVzc2FnZShzZXNzaW9uSWQ6IHN0cmluZywgY29udGVudDogc3RyaW5nKTogdm9pZCB7XG4gICAgY29uc3Qgc2Vzc2lvbiA9IHRoaXMuc2Vzc2lvbnMuZ2V0KHNlc3Npb25JZCk7XG4gICAgaWYgKCFzZXNzaW9uKSB7XG4gICAgICB0aHJvdyBuZXcgRXJyb3IoYFNlc3Npb24gbm90IGZvdW5kOiAke3Nlc3Npb25JZH1gKTtcbiAgICB9XG5cbiAgICBzZXNzaW9uLm1lc3NhZ2VzLnB1c2goe1xuICAgICAgcm9sZTogJ3N5c3RlbScsXG4gICAgICBjb250ZW50LFxuICAgICAgdGltZXN0YW1wOiBuZXcgRGF0ZSgpLFxuICAgIH0pO1xuICAgIHNlc3Npb24ubWVzc2FnZUNvdW50ID0gc2Vzc2lvbi5tZXNzYWdlcy5sZW5ndGg7XG4gIH1cblxuICAvKipcbiAgICogQWRkIGNvbnRleHQgdG8gc2Vzc2lvbiAocGVyc2lzdGVkIHRvIG1lbW9yeSlcbiAgICovXG4gIGFkZENvbnRleHQoc2Vzc2lvbklkOiBzdHJpbmcsIGNvbnRleHQ6IHN0cmluZyk6IG51bWJlciB7XG4gICAgY29uc3Qgc2Vzc2lvbiA9IHRoaXMuc2Vzc2lvbnMuZ2V0KHNlc3Npb25JZCk7XG4gICAgaWYgKCFzZXNzaW9uKSB7XG4gICAgICB0aHJvdyBuZXcgRXJyb3IoYFNlc3Npb24gbm90IGZvdW5kOiAke3Nlc3Npb25JZH1gKTtcbiAgICB9XG5cbiAgICBzZXNzaW9uLmNvbnRleHQucHVzaChjb250ZXh0KTtcblxuICAgIC8vIEFsc28gc3RvcmUgaW4gbWVtb3J5IGZvciByZXRyaWV2YWxcbiAgICBjb25zdCBtZW1vcnlJZCA9IHRoaXMubGxtLmFkZE1lbW9yeShjb250ZXh0LCB7XG4gICAgICBzZXNzaW9uSWQsXG4gICAgICB0eXBlOiAnY29udGV4dCcsXG4gICAgICB0aW1lc3RhbXA6IG5ldyBEYXRlKCkudG9JU09TdHJpbmcoKSxcbiAgICB9KTtcblxuICAgIHNlc3Npb24uYWN0aXZlTWVtb3J5SWRzLnB1c2gobWVtb3J5SWQpO1xuICAgIHJldHVybiBtZW1vcnlJZDtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgY29udmVyc2F0aW9uIGhpc3RvcnlcbiAgICovXG4gIGdldEhpc3Rvcnkoc2Vzc2lvbklkOiBzdHJpbmcsIGxpbWl0PzogbnVtYmVyKTogQ29udmVyc2F0aW9uTWVzc2FnZVtdIHtcbiAgICBjb25zdCBzZXNzaW9uID0gdGhpcy5zZXNzaW9ucy5nZXQoc2Vzc2lvbklkKTtcbiAgICBpZiAoIXNlc3Npb24pIHtcbiAgICAgIHJldHVybiBbXTtcbiAgICB9XG5cbiAgICBjb25zdCBtZXNzYWdlcyA9IHNlc3Npb24ubWVzc2FnZXM7XG4gICAgcmV0dXJuIGxpbWl0ID8gbWVzc2FnZXMuc2xpY2UoLWxpbWl0KSA6IG1lc3NhZ2VzO1xuICB9XG5cbiAgLyoqXG4gICAqIENsZWFyIHNlc3Npb24gaGlzdG9yeSAoa2VlcCBzZXNzaW9uIGFjdGl2ZSlcbiAgICovXG4gIGNsZWFySGlzdG9yeShzZXNzaW9uSWQ6IHN0cmluZyk6IHZvaWQge1xuICAgIGNvbnN0IHNlc3Npb24gPSB0aGlzLnNlc3Npb25zLmdldChzZXNzaW9uSWQpO1xuICAgIGlmIChzZXNzaW9uKSB7XG4gICAgICBzZXNzaW9uLm1lc3NhZ2VzID0gW107XG4gICAgICBzZXNzaW9uLmNvbnRleHQgPSBbXTtcbiAgICAgIHNlc3Npb24ubWVzc2FnZUNvdW50ID0gMDtcbiAgICB9XG4gIH1cblxuICAvKipcbiAgICogRW5kIGFuZCBkZWxldGUgc2Vzc2lvblxuICAgKi9cbiAgZW5kKHNlc3Npb25JZDogc3RyaW5nKTogYm9vbGVhbiB7XG4gICAgcmV0dXJuIHRoaXMuc2Vzc2lvbnMuZGVsZXRlKHNlc3Npb25JZCk7XG4gIH1cblxuICAvKipcbiAgICogTGlzdCBhbGwgYWN0aXZlIHNlc3Npb25zXG4gICAqL1xuICBsaXN0KCk6IENvbnZlcnNhdGlvblNlc3Npb25bXSB7XG4gICAgcmV0dXJuIEFycmF5LmZyb20odGhpcy5zZXNzaW9ucy52YWx1ZXMoKSk7XG4gIH1cblxuICAvKipcbiAgICogRXhwb3J0IHNlc3Npb24gYXMgSlNPTlxuICAgKi9cbiAgZXhwb3J0KHNlc3Npb25JZDogc3RyaW5nKTogc3RyaW5nIHwgbnVsbCB7XG4gICAgY29uc3Qgc2Vzc2lvbiA9IHRoaXMuc2Vzc2lvbnMuZ2V0KHNlc3Npb25JZCk7XG4gICAgaWYgKCFzZXNzaW9uKSB7XG4gICAgICByZXR1cm4gbnVsbDtcbiAgICB9XG5cbiAgICByZXR1cm4gSlNPTi5zdHJpbmdpZnkoc2Vzc2lvbiwgbnVsbCwgMik7XG4gIH1cblxuICAvKipcbiAgICogSW1wb3J0IHNlc3Npb24gZnJvbSBKU09OXG4gICAqL1xuICBpbXBvcnQoanNvbjogc3RyaW5nKTogQ29udmVyc2F0aW9uU2Vzc2lvbiB7XG4gICAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UoanNvbik7XG4gICAgY29uc3Qgc2Vzc2lvbjogQ29udmVyc2F0aW9uU2Vzc2lvbiA9IHtcbiAgICAgIC4uLmRhdGEsXG4gICAgICBjcmVhdGVkQXQ6IG5ldyBEYXRlKGRhdGEuY3JlYXRlZEF0KSxcbiAgICAgIG1lc3NhZ2VzOiBkYXRhLm1lc3NhZ2VzLm1hcCgobTogQ29udmVyc2F0aW9uTWVzc2FnZSkgPT4gKHtcbiAgICAgICAgLi4ubSxcbiAgICAgICAgdGltZXN0YW1wOiBuZXcgRGF0ZShtLnRpbWVzdGFtcCksXG4gICAgICB9KSksXG4gICAgfTtcblxuICAgIHRoaXMuc2Vzc2lvbnMuc2V0KHNlc3Npb24uaWQsIHNlc3Npb24pO1xuICAgIHJldHVybiBzZXNzaW9uO1xuICB9XG5cbiAgLyoqXG4gICAqIEJ1aWxkIGNvbnRleHQgc3RyaW5nIGZyb20gcmVjZW50IG1lc3NhZ2VzXG4gICAqL1xuICBwcml2YXRlIGJ1aWxkQ29udGV4dChzZXNzaW9uOiBDb252ZXJzYXRpb25TZXNzaW9uLCBtYXhNZXNzYWdlcyA9IDEwKTogc3RyaW5nIHtcbiAgICBjb25zdCByZWNlbnQgPSBzZXNzaW9uLm1lc3NhZ2VzLnNsaWNlKC1tYXhNZXNzYWdlcyk7XG4gICAgaWYgKHJlY2VudC5sZW5ndGggPT09IDApIHtcbiAgICAgIHJldHVybiAnJztcbiAgICB9XG5cbiAgICBjb25zdCBjb250ZXh0UGFydHM6IHN0cmluZ1tdID0gW107XG5cbiAgICAvLyBBZGQgcGVyc2lzdGVudCBjb250ZXh0XG4gICAgaWYgKHNlc3Npb24uY29udGV4dC5sZW5ndGggPiAwKSB7XG4gICAgICBjb250ZXh0UGFydHMucHVzaCgnQ29udGV4dDpcXG4nICsgc2Vzc2lvbi5jb250ZXh0LmpvaW4oJ1xcbicpKTtcbiAgICB9XG5cbiAgICAvLyBBZGQgY29udmVyc2F0aW9uIGhpc3RvcnlcbiAgICBjb25zdCBoaXN0b3J5ID0gcmVjZW50XG4gICAgICAubWFwKG0gPT4ge1xuICAgICAgICBjb25zdCByb2xlID0gbS5yb2xlID09PSAndXNlcicgPyAnVXNlcicgOiBtLnJvbGUgPT09ICdhc3Npc3RhbnQnID8gJ0Fzc2lzdGFudCcgOiAnU3lzdGVtJztcbiAgICAgICAgcmV0dXJuIGAke3JvbGV9OiAke20uY29udGVudH1gO1xuICAgICAgfSlcbiAgICAgIC5qb2luKCdcXG4nKTtcblxuICAgIGlmIChoaXN0b3J5KSB7XG4gICAgICBjb250ZXh0UGFydHMucHVzaCgnQ29udmVyc2F0aW9uOlxcbicgKyBoaXN0b3J5KTtcbiAgICB9XG5cbiAgICByZXR1cm4gY29udGV4dFBhcnRzLmpvaW4oJ1xcblxcbicpO1xuICB9XG59XG4iXX0=