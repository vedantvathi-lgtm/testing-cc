/**
 * Normalize a user-pasted Reflexio URL down to just the origin (scheme +
 * host + port). The dashboard proxy appends `/api/...` itself, so if the
 * user pastes `http://localhost:8071/api` we must strip the path —
 * otherwise FastAPI receives `/api/api/...` and returns 404.
 *
 * Returns null for empty input or anything the URL parser rejects.
 */
export function originOnly(raw: string): string | null {
  const candidate = raw.trim();
  if (!candidate) return null;
  try {
    return new URL(candidate).origin;
  } catch {
    return null;
  }
}
