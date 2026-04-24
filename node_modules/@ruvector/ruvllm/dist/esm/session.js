/**
 * Session Management for multi-turn conversations
 */
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
export class SessionManager {
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
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoic2Vzc2lvbi5qcyIsInNvdXJjZVJvb3QiOiIiLCJzb3VyY2VzIjpbIi4uLy4uL3NyYy9zZXNzaW9uLnRzIl0sIm5hbWVzIjpbXSwibWFwcGluZ3MiOiJBQUFBOztHQUVHO0FBU0g7Ozs7Ozs7Ozs7Ozs7Ozs7OztHQWtCRztBQUNILE1BQU0sT0FBTyxjQUFjO0lBSXpCLFlBQVksR0FBc0o7UUFIMUosYUFBUSxHQUFxQyxJQUFJLEdBQUcsRUFBRSxDQUFDO1FBSTdELElBQUksQ0FBQyxHQUFHLEdBQUcsR0FBRyxDQUFDO0lBQ2pCLENBQUM7SUFFRDs7T0FFRztJQUNILE1BQU0sQ0FBQyxRQUFrQztRQUN2QyxNQUFNLEVBQUUsR0FBRyxXQUFXLElBQUksQ0FBQyxHQUFHLEVBQUUsSUFBSSxJQUFJLENBQUMsTUFBTSxFQUFFLENBQUMsUUFBUSxDQUFDLEVBQUUsQ0FBQyxDQUFDLEtBQUssQ0FBQyxDQUFDLEVBQUUsQ0FBQyxDQUFDLEVBQUUsQ0FBQztRQUM3RSxNQUFNLE9BQU8sR0FBd0I7WUFDbkMsRUFBRTtZQUNGLFNBQVMsRUFBRSxJQUFJLElBQUksRUFBRTtZQUNyQixZQUFZLEVBQUUsQ0FBQztZQUNmLFFBQVEsRUFBRSxFQUFFO1lBQ1osT0FBTyxFQUFFLEVBQUU7WUFDWCxlQUFlLEVBQUUsRUFBRTtZQUNuQixRQUFRLEVBQUUsUUFBUSxJQUFJLEVBQUU7U0FDekIsQ0FBQztRQUNGLElBQUksQ0FBQyxRQUFRLENBQUMsR0FBRyxDQUFDLEVBQUUsRUFBRSxPQUFPLENBQUMsQ0FBQztRQUMvQixPQUFPLE9BQU8sQ0FBQztJQUNqQixDQUFDO0lBRUQ7O09BRUc7SUFDSCxHQUFHLENBQUMsU0FBaUI7UUFDbkIsT0FBTyxJQUFJLENBQUMsUUFBUSxDQUFDLEdBQUcsQ0FBQyxTQUFTLENBQUMsQ0FBQztJQUN0QyxDQUFDO0lBRUQ7O09BRUc7SUFDSCxJQUFJLENBQUMsU0FBaUIsRUFBRSxPQUFlLEVBQUUsTUFBeUI7UUFDaEUsTUFBTSxPQUFPLEdBQUcsSUFBSSxDQUFD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QUFDLElBQUksS0FBSyxNQUFNLENBQUMsQ0FBQyxDQUFDLE1BQU0sQ0FBQyxDQUFDLENBQUMsQ0FBQyxDQUFDLElBQUksS0FBSyxXQUFXLENBQUMsQ0FBQyxDQUFDLFdBQVcsQ0FBQyxDQUFDLENBQUMsUUFBUSxDQUFDO1lBQzFGLE9BQU8sR0FBRyxJQUFJLEtBQUssQ0FBQyxDQUFDLE9BQU8sRUFBRSxDQUFDO1FBQ2pDLENBQUMsQ0FBQzthQUNELElBQUksQ0FBQyxJQUFJLENBQUMsQ0FBQztRQUVkLElBQUksT0FBTyxFQUFFLENBQUM7WUFDWixZQUFZLENBQUMsSUFBSSxDQUFDLGlCQUFpQixHQUFHLE9BQU8sQ0FBQyxDQUFDO1FBQ2pELENBQUM7UUFFRCxPQUFPLFlBQVksQ0FBQyxJQUFJLENBQUMsTUFBTSxDQUFDLENBQUM7SUFDbkMsQ0FBQztDQUNGIiwic291cmNlc0NvbnRlbnQiOlsiLyoqXG4gKiBTZXNzaW9uIE1hbmFnZW1lbnQgZm9yIG11bHRpLXR1cm4gY29udmVyc2F0aW9uc1xuICovXG5cbmltcG9ydCB7XG4gIENvbnZlcnNhdGlvblNlc3Npb24sXG4gIENvbnZlcnNhdGlvbk1lc3NhZ2UsXG4gIFF1ZXJ5UmVzcG9uc2UsXG4gIEdlbmVyYXRpb25Db25maWcsXG59IGZyb20gJy4vdHlwZXMnO1xuXG4vKipcbiAqIFNlc3Npb24gTWFuYWdlciBmb3IgbXVsdGktdHVybiBjb252ZXJzYXRpb25zXG4gKlxuICogQGV4YW1wbGVcbiAqIGBgYHR5cGVzY3JpcHRcbiAqIGltcG9ydCB7IFJ1dkxMTSwgU2Vzc2lvbk1hbmFnZXIgfSBmcm9tICdAcnV2ZWN0b3IvcnV2bGxtJztcbiAqXG4gKiBjb25zdCBsbG0gPSBuZXcgUnV2TExNKCk7XG4gKiBjb25zdCBzZXNzaW9ucyA9IG5ldyBTZXNzaW9uTWFuYWdlcihsbG0pO1xuICpcbiAqIC8vIENyZWF0ZSBhIG5ldyBzZXNzaW9uXG4gKiBjb25zdCBzZXNzaW9uID0gc2Vzc2lvbnMuY3JlYXRlKCk7XG4gKlxuICogLy8gQ2hhdCB3aXRoIGNvbnRleHRcbiAqIGNvbnN0IHJlc3BvbnNlMSA9IHNlc3Npb25zLmNoYXQoc2Vzc2lvbi5pZCwgJ1doYXQgaXMgUHl0aG9uPycpO1xuICogY29uc3QgcmVzcG9uc2UyID0gc2Vzc2lvbnMuY2hhdChzZXNzaW9uLmlkLCAnSG93IGRvIEkgaW5zdGFsbCBpdD8nKTtcbiAqIC8vIFNlY29uZCBxdWVyeSBhdXRvbWF0aWNhbGx5IGhhcyBjb250ZXh0IGZyb20gZmlyc3RcbiAqIGBgYFxuICovXG5leHBvcnQgY2xhc3MgU2Vzc2lvbk1hbmFnZXIge1xuICBwcml2YXRlIHNlc3Npb25zOiBNYXA8c3RyaW5nLCBDb252ZXJzYXRpb25TZXNzaW9uPiA9IG5ldyBNYXAoKTtcbiAgcHJpdmF0ZSBsbG06IHsgcXVlcnk6ICh0ZXh0OiBzdHJpbmcsIGNvbmZpZz86IEdlbmVyYXRpb25Db25maWcpID0+IFF1ZXJ5UmVzcG9uc2U7IGFkZE1lbW9yeTogKGNvbnRlbnQ6IHN0cmluZywgbWV0YWRhdGE/OiBSZWNvcmQ8c3RyaW5nLCB1bmtub3duPikgPT4gbnVtYmVyIH07XG5cbiAgY29uc3RydWN0b3IobGxtOiB7IHF1ZXJ5OiAodGV4dDogc3RyaW5nLCBjb25maWc/OiBHZW5lcmF0aW9uQ29uZmlnKSA9PiBRdWVyeVJlc3BvbnNlOyBhZGRNZW1vcnk6IChjb250ZW50OiBzdHJpbmcsIG1ldGFkYXRhPzogUmVjb3JkPHN0cmluZywgdW5rbm93bj4pID0+IG51bWJlciB9KSB7XG4gICAgdGhpcy5sbG0gPSBsbG07XG4gIH1cblxuICAvKipcbiAgICogQ3JlYXRlIGEgbmV3IGNvbnZlcnNhdGlvbiBzZXNzaW9uXG4gICAqL1xuICBjcmVhdGUobWV0YWRhdGE/OiBSZWNvcmQ8c3RyaW5nLCB1bmtub3duPik6IENvbnZlcnNhdGlvblNlc3Npb24ge1xuICAgIGNvbnN0IGlkID0gYHNlc3Npb24tJHtEYXRlLm5vdygpfS0ke01hdGgucmFuZG9tKCkudG9TdHJpbmcoMzYpLnNsaWNlKDIsIDgpfWA7XG4gICAgY29uc3Qgc2Vzc2lvbjogQ29udmVyc2F0aW9uU2Vzc2lvbiA9IHtcbiAgICAgIGlkLFxuICAgICAgY3JlYXRlZEF0OiBuZXcgRGF0ZSgpLFxuICAgICAgbWVzc2FnZUNvdW50OiAwLFxuICAgICAgbWVzc2FnZXM6IFtdLFxuICAgICAgY29udGV4dDogW10sXG4gICAgICBhY3RpdmVNZW1vcnlJZHM6IFtdLFxuICAgICAgbWV0YWRhdGE6IG1ldGFkYXRhID8/IHt9LFxuICAgIH07XG4gICAgdGhpcy5zZXNzaW9ucy5zZXQoaWQsIHNlc3Npb24pO1xuICAgIHJldHVybiBzZXNzaW9uO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCBzZXNzaW9uIGJ5IElEXG4gICAqL1xuICBnZXQoc2Vzc2lvbklkOiBzdHJpbmcpOiBDb252ZXJzYXRpb25TZXNzaW9uIHwgdW5kZWZpbmVkIHtcbiAgICByZXR1cm4gdGhpcy5zZXNzaW9ucy5nZXQoc2Vzc2lvbklkKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBDaGF0IHdpdGhpbiBhIHNlc3Npb24gKG1haW50YWlucyBjb250ZXh0KVxuICAgKi9cbiAgY2hhdChzZXNzaW9uSWQ6IHN0cmluZywgbWVzc2FnZTogc3RyaW5nLCBjb25maWc/OiBHZW5lcmF0aW9uQ29uZmlnKTogUXVlcnlSZXNwb25zZSB7XG4gICAgY29uc3Qgc2Vzc2lvbiA9IHRoaXMuc2Vzc2lvbnMuZ2V0KHNlc3Npb25JZCk7XG4gICAgaWYgKCFzZXNzaW9uKSB7XG4gICAgICB0aHJvdyBuZXcgRXJyb3IoYFNlc3Npb24gbm90IGZvdW5kOiAke3Nlc3Npb25JZH1gKTtcbiAgICB9XG5cbiAgICAvLyBBZGQgdXNlciBtZXNzYWdlXG4gICAgc2Vzc2lvbi5tZXNzYWdlcy5wdXNoKHtcbiAgICAgIHJvbGU6ICd1c2VyJyxcbiAgICAgIGNvbnRlbnQ6IG1lc3NhZ2UsXG4gICAgICB0aW1lc3RhbXA6IG5ldyBEYXRlKCksXG4gICAgfSk7XG5cbiAgICAvLyBCdWlsZCBjb250ZXh0IGZyb20gcmVjZW50IG1lc3NhZ2VzXG4gICAgY29uc3QgY29udGV4dFdpbmRvdyA9IHRoaXMuYnVpbGRDb250ZXh0KHNlc3Npb24pO1xuXG4gICAgLy8gUXVlcnkgd2l0aCBjb250ZXh0XG4gICAgY29uc3QgcHJvbXB0ID0gY29udGV4dFdpbmRvdyA/IGAke2NvbnRleHRXaW5kb3d9XFxuXFxuVXNlcjogJHttZXNzYWdlfWAgOiBtZXNzYWdlO1xuICAgIGNvbnN0IHJlc3BvbnNlID0gdGhpcy5sbG0ucXVlcnkocHJvbXB0LCBjb25maWcpO1xuXG4gICAgLy8gQWRkIGFzc2lzdGFudCByZXNwb25zZVxuICAgIHNlc3Npb24ubWVzc2FnZXMucHVzaCh7XG4gICAgICByb2xlOiAnYXNzaXN0YW50JyxcbiAgICAgIGNvbnRlbnQ6IHJlc3BvbnNlLnRleHQsXG4gICAgICB0aW1lc3RhbXA6IG5ldyBEYXRlKCksXG4gICAgICByZXF1ZXN0SWQ6IHJlc3BvbnNlLnJlcXVlc3RJZCxcbiAgICB9KTtcblxuICAgIHNlc3Npb24ubWVzc2FnZUNvdW50ID0gc2Vzc2lvbi5tZXNzYWdlcy5sZW5ndGg7XG5cbiAgICByZXR1cm4gcmVzcG9uc2U7XG4gIH1cblxuICAvKipcbiAgICogQWRkIHN5c3RlbSBtZXNzYWdlIHRvIHNlc3Npb25cbiAgICovXG4gIGFkZFN5c3RlbU1lc3NhZ2Uoc2Vzc2lvbklkOiBzdHJpbmcsIGNvbnRlbnQ6IHN0cmluZyk6IHZvaWQge1xuICAgIGNvbnN0IHNlc3Npb24gPSB0aGlzLnNlc3Npb25zLmdldChzZXNzaW9uSWQpO1xuICAgIGlmICghc2Vzc2lvbikge1xuICAgICAgdGhyb3cgbmV3IEVycm9yKGBTZXNzaW9uIG5vdCBmb3VuZDogJHtzZXNzaW9uSWR9YCk7XG4gICAgfVxuXG4gICAgc2Vzc2lvbi5tZXNzYWdlcy5wdXNoKHtcbiAgICAgIHJvbGU6ICdzeXN0ZW0nLFxuICAgICAgY29udGVudCxcbiAgICAgIHRpbWVzdGFtcDogbmV3IERhdGUoKSxcbiAgICB9KTtcbiAgICBzZXNzaW9uLm1lc3NhZ2VDb3VudCA9IHNlc3Npb24ubWVzc2FnZXMubGVuZ3RoO1xuICB9XG5cbiAgLyoqXG4gICAqIEFkZCBjb250ZXh0IHRvIHNlc3Npb24gKHBlcnNpc3RlZCB0byBtZW1vcnkpXG4gICAqL1xuICBhZGRDb250ZXh0KHNlc3Npb25JZDogc3RyaW5nLCBjb250ZXh0OiBzdHJpbmcpOiBudW1iZXIge1xuICAgIGNvbnN0IHNlc3Npb24gPSB0aGlzLnNlc3Npb25zLmdldChzZXNzaW9uSWQpO1xuICAgIGlmICghc2Vzc2lvbikge1xuICAgICAgdGhyb3cgbmV3IEVycm9yKGBTZXNzaW9uIG5vdCBmb3VuZDogJHtzZXNzaW9uSWR9YCk7XG4gICAgfVxuXG4gICAgc2Vzc2lvbi5jb250ZXh0LnB1c2goY29udGV4dCk7XG5cbiAgICAvLyBBbHNvIHN0b3JlIGluIG1lbW9yeSBmb3IgcmV0cmlldmFsXG4gICAgY29uc3QgbWVtb3J5SWQgPSB0aGlzLmxsbS5hZGRNZW1vcnkoY29udGV4dCwge1xuICAgICAgc2Vzc2lvbklkLFxuICAgICAgdHlwZTogJ2NvbnRleHQnLFxuICAgICAgdGltZXN0YW1wOiBuZXcgRGF0ZSgpLnRvSVNPU3RyaW5nKCksXG4gICAgfSk7XG5cbiAgICBzZXNzaW9uLmFjdGl2ZU1lbW9yeUlkcy5wdXNoKG1lbW9yeUlkKTtcbiAgICByZXR1cm4gbWVtb3J5SWQ7XG4gIH1cblxuICAvKipcbiAgICogR2V0IGNvbnZlcnNhdGlvbiBoaXN0b3J5XG4gICAqL1xuICBnZXRIaXN0b3J5KHNlc3Npb25JZDogc3RyaW5nLCBsaW1pdD86IG51bWJlcik6IENvbnZlcnNhdGlvbk1lc3NhZ2VbXSB7XG4gICAgY29uc3Qgc2Vzc2lvbiA9IHRoaXMuc2Vzc2lvbnMuZ2V0KHNlc3Npb25JZCk7XG4gICAgaWYgKCFzZXNzaW9uKSB7XG4gICAgICByZXR1cm4gW107XG4gICAgfVxuXG4gICAgY29uc3QgbWVzc2FnZXMgPSBzZXNzaW9uLm1lc3NhZ2VzO1xuICAgIHJldHVybiBsaW1pdCA/IG1lc3NhZ2VzLnNsaWNlKC1saW1pdCkgOiBtZXNzYWdlcztcbiAgfVxuXG4gIC8qKlxuICAgKiBDbGVhciBzZXNzaW9uIGhpc3RvcnkgKGtlZXAgc2Vzc2lvbiBhY3RpdmUpXG4gICAqL1xuICBjbGVhckhpc3Rvcnkoc2Vzc2lvbklkOiBzdHJpbmcpOiB2b2lkIHtcbiAgICBjb25zdCBzZXNzaW9uID0gdGhpcy5zZXNzaW9ucy5nZXQoc2Vzc2lvbklkKTtcbiAgICBpZiAoc2Vzc2lvbikge1xuICAgICAgc2Vzc2lvbi5tZXNzYWdlcyA9IFtdO1xuICAgICAgc2Vzc2lvbi5jb250ZXh0ID0gW107XG4gICAgICBzZXNzaW9uLm1lc3NhZ2VDb3VudCA9IDA7XG4gICAgfVxuICB9XG5cbiAgLyoqXG4gICAqIEVuZCBhbmQgZGVsZXRlIHNlc3Npb25cbiAgICovXG4gIGVuZChzZXNzaW9uSWQ6IHN0cmluZyk6IGJvb2xlYW4ge1xuICAgIHJldHVybiB0aGlzLnNlc3Npb25zLmRlbGV0ZShzZXNzaW9uSWQpO1xuICB9XG5cbiAgLyoqXG4gICAqIExpc3QgYWxsIGFjdGl2ZSBzZXNzaW9uc1xuICAgKi9cbiAgbGlzdCgpOiBDb252ZXJzYXRpb25TZXNzaW9uW10ge1xuICAgIHJldHVybiBBcnJheS5mcm9tKHRoaXMuc2Vzc2lvbnMudmFsdWVzKCkpO1xuICB9XG5cbiAgLyoqXG4gICAqIEV4cG9ydCBzZXNzaW9uIGFzIEpTT05cbiAgICovXG4gIGV4cG9ydChzZXNzaW9uSWQ6IHN0cmluZyk6IHN0cmluZyB8IG51bGwge1xuICAgIGNvbnN0IHNlc3Npb24gPSB0aGlzLnNlc3Npb25zLmdldChzZXNzaW9uSWQpO1xuICAgIGlmICghc2Vzc2lvbikge1xuICAgICAgcmV0dXJuIG51bGw7XG4gICAgfVxuXG4gICAgcmV0dXJuIEpTT04uc3RyaW5naWZ5KHNlc3Npb24sIG51bGwsIDIpO1xuICB9XG5cbiAgLyoqXG4gICAqIEltcG9ydCBzZXNzaW9uIGZyb20gSlNPTlxuICAgKi9cbiAgaW1wb3J0KGpzb246IHN0cmluZyk6IENvbnZlcnNhdGlvblNlc3Npb24ge1xuICAgIGNvbnN0IGRhdGEgPSBKU09OLnBhcnNlKGpzb24pO1xuICAgIGNvbnN0IHNlc3Npb246IENvbnZlcnNhdGlvblNlc3Npb24gPSB7XG4gICAgICAuLi5kYXRhLFxuICAgICAgY3JlYXRlZEF0OiBuZXcgRGF0ZShkYXRhLmNyZWF0ZWRBdCksXG4gICAgICBtZXNzYWdlczogZGF0YS5tZXNzYWdlcy5tYXAoKG06IENvbnZlcnNhdGlvbk1lc3NhZ2UpID0+ICh7XG4gICAgICAgIC4uLm0sXG4gICAgICAgIHRpbWVzdGFtcDogbmV3IERhdGUobS50aW1lc3RhbXApLFxuICAgICAgfSkpLFxuICAgIH07XG5cbiAgICB0aGlzLnNlc3Npb25zLnNldChzZXNzaW9uLmlkLCBzZXNzaW9uKTtcbiAgICByZXR1cm4gc2Vzc2lvbjtcbiAgfVxuXG4gIC8qKlxuICAgKiBCdWlsZCBjb250ZXh0IHN0cmluZyBmcm9tIHJlY2VudCBtZXNzYWdlc1xuICAgKi9cbiAgcHJpdmF0ZSBidWlsZENvbnRleHQoc2Vzc2lvbjogQ29udmVyc2F0aW9uU2Vzc2lvbiwgbWF4TWVzc2FnZXMgPSAxMCk6IHN0cmluZyB7XG4gICAgY29uc3QgcmVjZW50ID0gc2Vzc2lvbi5tZXNzYWdlcy5zbGljZSgtbWF4TWVzc2FnZXMpO1xuICAgIGlmIChyZWNlbnQubGVuZ3RoID09PSAwKSB7XG4gICAgICByZXR1cm4gJyc7XG4gICAgfVxuXG4gICAgY29uc3QgY29udGV4dFBhcnRzOiBzdHJpbmdbXSA9IFtdO1xuXG4gICAgLy8gQWRkIHBlcnNpc3RlbnQgY29udGV4dFxuICAgIGlmIChzZXNzaW9uLmNvbnRleHQubGVuZ3RoID4gMCkge1xuICAgICAgY29udGV4dFBhcnRzLnB1c2goJ0NvbnRleHQ6XFxuJyArIHNlc3Npb24uY29udGV4dC5qb2luKCdcXG4nKSk7XG4gICAgfVxuXG4gICAgLy8gQWRkIGNvbnZlcnNhdGlvbiBoaXN0b3J5XG4gICAgY29uc3QgaGlzdG9yeSA9IHJlY2VudFxuICAgICAgLm1hcChtID0+IHtcbiAgICAgICAgY29uc3Qgcm9sZSA9IG0ucm9sZSA9PT0gJ3VzZXInID8gJ1VzZXInIDogbS5yb2xlID09PSAnYXNzaXN0YW50JyA/ICdBc3Npc3RhbnQnIDogJ1N5c3RlbSc7XG4gICAgICAgIHJldHVybiBgJHtyb2xlfTogJHttLmNvbnRlbnR9YDtcbiAgICAgIH0pXG4gICAgICAuam9pbignXFxuJyk7XG5cbiAgICBpZiAoaGlzdG9yeSkge1xuICAgICAgY29udGV4dFBhcnRzLnB1c2goJ0NvbnZlcnNhdGlvbjpcXG4nICsgaGlzdG9yeSk7XG4gICAgfVxuXG4gICAgcmV0dXJuIGNvbnRleHRQYXJ0cy5qb2luKCdcXG5cXG4nKTtcbiAgfVxufVxuIl19