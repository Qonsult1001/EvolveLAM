---
name: code-web
description: Web development patterns — REST APIs, TypeScript, React, frontend/backend
tools: [bash, read_file, write_file, edit_file]
---

# Web Development Patterns

## REST API Design

- Use nouns for resources (`/users`, `/posts`), verbs come from HTTP methods
- GET = read, POST = create, PUT = full update, PATCH = partial update, DELETE = remove
- Return appropriate status codes: 200 OK, 201 Created, 400 Bad Request, 404 Not Found, 500 Server Error
- Pagination: use `?page=1&limit=20` or cursor-based for large datasets
- Version APIs: `/api/v1/...`

## TypeScript

- Prefer `interface` for object shapes, `type` for unions and intersections
- Use `strict: true` in tsconfig — catches real bugs
- Avoid `any` — use `unknown` when type is truly unknown, then narrow with type guards
- Template literal types for string patterns: `type Route = \`/api/${string}\``

## React Patterns

- Functional components + hooks (not class components)
- `useState` for local state, `useReducer` for complex state logic
- `useEffect` with proper dependency arrays — empty `[]` for mount-only
- Custom hooks to extract reusable logic: `useAuth()`, `useFetch()`
- Avoid prop drilling — use Context or state management for deep trees

## Authentication

- JWT for stateless auth, session tokens for stateful
- Store tokens in httpOnly cookies (not localStorage) to prevent XSS
- Refresh token rotation for long-lived sessions
- Always validate tokens server-side

## CSS/Styling

- CSS modules or Tailwind for scoped styles
- Flexbox for 1D layout, Grid for 2D layout
- Mobile-first responsive design with `min-width` breakpoints

## Security

- Sanitize all user input
- Use parameterized queries (never string concatenation for SQL)
- Set CORS headers appropriately
- Content Security Policy headers
- Rate limiting on auth endpoints

## Patterns Learned

*(This section grows as the agent encounters and solves real web problems)*
