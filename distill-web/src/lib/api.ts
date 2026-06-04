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
	localStorage.removeItem('distill_user_id');
}

export function isLoggedIn(): boolean {
	return !!getToken();
}

export function getUserId(): string | null {
	if (typeof window === 'undefined') return null;
	return localStorage.getItem('distill_user_id');
}

export function setUserId(id: string) {
	localStorage.setItem('distill_user_id', id);
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
	body: string;
	author_id: string | null;
	last_editor_id: string | null;
	last_editor_name: string | null;
	last_editor_role: string | null;
	is_stale: boolean;
	created_at: string;
	updated_at: string;
	rating_count: number;
	rating_avg: number | null;
	rating_count_since_edit: number;
	rating_avg_since_edit: number | null;
}

export interface Discussion {
	id: string;
	question_id: string;
	parent_id: string | null;
	author_id: string;
	author_name: string;
	author_role: string;
	author_avatar: string | null;
	body: string;
	depth: number;
	is_deleted: boolean;
	score: number;
	user_vote: number | null;
	created_at: string;
}

export interface IndividualAnswer {
	id: string;
	question_id: string;
	author_id: string;
	author_name: string;
	author_role: string;
	body: string;
	is_accepted: boolean;
	rating_avg: number | null;
	rating_count: number;
	created_at: string;
	updated_at: string;
}

export interface VoteResult {
	score: number;
	user_vote: number | null;
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
	search: (query: string) => request<SearchResult[]>('GET', `/questions/search?q=${encodeURIComponent(query)}`),

	// Wiki Answer
	getWikiAnswer: (questionId: string) =>
		request<Answer>('GET', `/questions/${questionId}/wiki-answer`),
	editWikiAnswer: (questionId: string, body: string, editMessage?: string) =>
		request<Answer>('PUT', `/questions/${questionId}/wiki-answer`, { body, edit_message: editMessage }),
	getWikiHistory: (questionId: string) =>
		request<{ id: string; editor_id: string; edit_message: string | null; created_at: string }[]>('GET', `/questions/${questionId}/wiki-answer/history`),
	getRevision: (revisionId: string) =>
		request<{ id: string; editor_id: string; body: string; diff: string; edit_message: string | null; created_at: string }>('GET', `/revisions/${revisionId}`),
	rateWikiAnswer: (wikiAnswerId: string, score: number) =>
		request<{ id: string; rater_id: string; score: number }>('POST', `/wiki-answers/${wikiAnswerId}/ratings`, { score }),
	deleteWikiRating: (wikiAnswerId: string) =>
		request<void>('DELETE', `/wiki-answers/${wikiAnswerId}/ratings/mine`),
	getWikiRatings: (wikiAnswerId: string) =>
		request<{ id: string; rater_id: string; score: number }[]>('GET', `/wiki-answers/${wikiAnswerId}/ratings`),

	// Discussions
	listDiscussions: (questionId: string, parentId?: string, answerId?: string) => {
		const params = new URLSearchParams();
		if (parentId) params.set('parent_id', parentId);
		if (answerId) params.set('answer_id', answerId);
		const qs = params.toString();
		return request<Discussion[]>('GET', `/questions/${questionId}/discussions${qs ? '?' + qs : ''}`);
	},
	createDiscussion: (questionId: string, body: string, parentId?: string, answerId?: string) =>
		request<Discussion>('POST', `/questions/${questionId}/discussions`, { body, parent_id: parentId, answer_id: answerId }),
	voteDiscussion: (discussionId: string, direction: number) =>
		request<VoteResult>('POST', `/discussions/${discussionId}/vote`, { direction }),

	// Tags
	listTags: () => request<{ tag: string; count: number }[]>('GET', '/tags'),

	// Individual answers
	listAnswers: (questionId: string) =>
		request<IndividualAnswer[]>('GET', `/questions/${questionId}/answers`),
	createAnswer: (questionId: string, body: string) =>
		request<IndividualAnswer>('POST', `/questions/${questionId}/answers`, { body }),
	rateAnswer: (answerId: string, score: number) =>
		request<{ rating_count: number; rating_avg: number | null; your_score: number | null }>('POST', `/answers/${answerId}/ratings`, { score }),
	deleteAnswerRating: (answerId: string) =>
		request<void>('DELETE', `/answers/${answerId}/ratings/mine`),
};
