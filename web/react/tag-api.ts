import { ApiClientError, type RequestJsonOptions } from "@xcss/web/http-client";
import type { AdministratorApiClient } from "@xcss/web/admin-web";
import { t } from "@xcss/web/admin-ui/i18n";

const queued = new WeakMap<AdministratorApiClient, Promise<unknown>>();

/** Consume each bounded tag response before admitting the next SQLite read. */
export function tagRequest<T>(
  client: AdministratorApiClient,
  path: string,
  guard: (value: unknown) => value is T,
  method?: string,
  body?: unknown,
  signal?: AbortSignal,
): Promise<T> {
  const result = (queued.get(client) ?? Promise.resolve()).then(async () => {
    const options: RequestJsonOptions = {
      method,
      timeoutMs: 5000,
      maxResponseBytes: 8 * 1024 * 1024,
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    };
    for (let attempt = 0; ; attempt++) {
      if (signal?.aborted)
        throw new DOMException("The view changed.", "AbortError");
      try {
        return await client.request(`/api/v1/file-tags${path}`, guard, options);
      } catch (cause: unknown) {
        if (cause instanceof ApiClientError && cause.status === 401)
          window.location.replace("/__xczs__/login");
        // Directory metadata and scans can briefly own the same connection.
        // Retry only explicit temporary read failures; never repeat mutations.
        if (
          (!method || method === "GET") &&
          attempt < 2 &&
          cause instanceof ApiClientError &&
          cause.retryable &&
          ((cause.status === 429 && cause.code === "too_many_requests") ||
            (cause.status === 503 && cause.code === "service_unavailable")) &&
          (cause.retryAfterSeconds ?? 1) <= 2
        ) {
          await new Promise((resolve) =>
            setTimeout(resolve, Math.max(0, cause.retryAfterSeconds ?? 1) * 1000),
          );
          continue;
        }
        throw cause;
      }
    }
  });
  queued.set(
    client,
    result.catch(() => {}),
  );
  return result;
}

export function tagFailure(cause: unknown): string {
  if (cause instanceof ApiClientError && cause.code === "capacity_exhausted")
    return t(
      "已达到持久容量限制，请先整理数据或备份文件。",
      "Storage capacity reached. Organize your data or backups before retrying.",
    );
  return t(
    "无法读取或更新标签，请稍后重试。",
    "Could not read or update tags. Please try again.",
  );
}
