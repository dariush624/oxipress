// Cloudflare Pages Function: serves the build's Markdown twin (index.md) when a client sends
// `Accept: text/markdown`; everyone else keeps getting HTML. Scoped by static/_routes.json.
export async function onRequest({ request, next, env }) {
  if (request.method !== "GET" && request.method !== "HEAD") return next();

  const wantsMarkdown = /(^|,)\s*text\/markdown\b/i.test(request.headers.get("Accept") || "");
  if (wantsMarkdown) {
    const url = new URL(request.url);
    url.pathname = url.pathname.replace(/\/?$/, "/") + "index.md";
    const asset = await env.ASSETS.fetch(new Request(url, { method: request.method }));
    if (asset.ok) {
      const headers = new Headers(asset.headers);
      headers.set("Content-Type", "text/markdown; charset=utf-8");
      headers.set("Vary", "Accept");
      return new Response(asset.body, { status: 200, headers });
    }
  }

  const response = await next();
  const headers = new Headers(response.headers);
  headers.append("Vary", "Accept");
  return new Response(response.body, { status: response.status, statusText: response.statusText, headers });
}
