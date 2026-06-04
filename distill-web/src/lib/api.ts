const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:3000';

function getToken(): string | null {
	if (typeof window === 'undefined') return null;
	return localStorage.getItem('distill_token');
}

export function setToken(token: string) {
	localStorage.setItem('distill_token', token);
}

export function clearToken() {
	localStorage.removeItem('distill_token');
}

export function isLoggedIn(): boolean {
	return !!getToken();
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
	const headers: Record<string, string> = { 'Content-Type': 'application/json' };
	const token = getToken();
	if (token) headers['Authorization'] = `Bearer ${token}`;

	const resp = await fetch(`${API_URL}${path}`, {
		method,
		headers,
		body: body ? JSON.stringify(body) : undefined
	});

	if (!resp.ok) {
		const text = await resp.text();
		throw new Error(`${resp.status}: ${text}`);
	}

	if (resp.status === 204) return undefined as T;
	return resp.json();
}

// Types
export interface Question {
	id: string;
	author_id: string;
	title: string;
	body: string;
	tags: string[];
	status: string;
	created_at: string;
}

export interface Answer {
	id: string;
	question_id: string;
	author_id: string | null;
	author_type: string;
	body: string;
	is_stale: boolean;
	created_at: string;
	rating_count: number;
	rating_avg: number | null;
	rating_positive_pct: number | null;
	comment_count: number;
}

export interface SearchResult {
	id: string;
	title: string;
	body: string;
	score: number;
	tags: string[];
	created_at: string;
}

export interface Comment {
	id: string;
	author_id: string;
	body: string;
	created_at: string;
}

export interface Paginated<T> {
	data: T[];
	next_cursor: string | null;
}

export interface AuthConfig {
	github_client_id: string;
	google_enabled: boolean;
}

// API methods
export const api = {
	// Auth
	getAuthConfig: () => request<AuthConfig>('GET', '/auth/config'),
	getMe: () => request<{ id: string; display_name: string; role: string; email?: string }>('GET', '/me'),
	exchangeToken: (code: string, device_code: string) =>
		request<{ token: string }>('POST', '/auth/token', { code, device_code }),

	// Questions
	listQuestions: (limit = 20, cursor?: string) => {
		const params = new URLSearchParams({ limit: String(limit) });
		if (cursor) params.set('after', cursor);
		return request<Paginated<Question>>('GET', `/questions?${params}`);
	},
	getQuestion: (id: string) => request<Question>('GET', `/questions/${id}`),
	createQuestion: (title: string, body: string, tags: string[], generate_ai_answer = false) =>
		request<Question>('POST', '/questions', { title, body, tags, generate_ai_answer }),
	search: (query: string) => request<SearchResult[]>('GET', `/search?q=${encodeURIComponent(query)}`),

	// Answers
	getAnswers: (questionId: string) => request<Answer[]>('GET', `/questions/${questionId}/answers`),
	createAnswer: (questionId: string, body: string) =>
		request<Answer>('POST', `/questions/${questionId}/answers`, { body }),
	rateAnswer: (answerId: string, score: number) =>
		request<void>('POST', `/answers/${answerId}/ratings`, { score }),

	// Comments
	getQuestionComments: (questionId: string) =>
		request<Paginated<Comment>>('GET', `/questions/${questionId}/comments`),
	getAnswerComments: (answerId: string) =>
		request<Paginated<Comment>>('GET', `/answers/${answerId}/comments`),
	createQuestionComment: (questionId: string, body: string) =>
		request<Comment>('POST', `/questions/${questionId}/comments`, { body }),
	createAnswerComment: (answerId: string, body: string) =>
		request<Comment>('POST', `/answers/${answerId}/comments`, { body }),

	// Tags
	listTags: () => request<{ tag: string; count: number }[]>('GET', '/tags'),
};
