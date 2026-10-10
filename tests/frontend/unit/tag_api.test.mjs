import assert from "node:assert/strict";
import test from "node:test";
import { ApiClientError } from "@xcss/web/http-client";
import { tagRequest } from "../../../web/react/tag-api.ts";

const busy = () => new ApiClientError({
  message: "Busy",
  status: 429,
  code: "too_many_requests",
  retryable: true,
  retryAfterSeconds: 0,
});
const accepted = value => value === "accepted";

test("temporary tag reads retry before admitting the next request", async () => {
  const requests = [];
  let reads = 0;
  const client = {
    async request(path) {
      requests.push(path);
      if (path.endsWith("/tags") && reads++ === 0) throw busy();
      return "accepted";
    },
  };
  await Promise.all([
    tagRequest(client, "/tags", accepted),
    tagRequest(client, "/status", accepted),
  ]);
  assert.deepEqual(requests, [
    "/api/v1/file-tags/tags",
    "/api/v1/file-tags/tags",
    "/api/v1/file-tags/status",
  ]);
});

test("tag retries are bounded and never repeat writes or permanent failures", async () => {
  let requests = 0;
  const client = {
    async request() {
      requests++;
      throw busy();
    },
  };
  await assert.rejects(tagRequest(client, "/tags", accepted));
  assert.equal(requests, 3);
  requests = 0;
  await assert.rejects(tagRequest(client, "/tags", accepted, "POST", {}));
  assert.equal(requests, 1);
  requests = 0;
  client.request = async () => {
    requests++;
    throw new ApiClientError({
      message: "Capacity reached",
      status: 503,
      code: "capacity_exhausted",
    });
  };
  await assert.rejects(tagRequest(client, "/tags", accepted));
  assert.equal(requests, 1);
});

test("obsolete queued tag reads do not issue requests", async () => {
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  const requests = [];
  const client = {
    async request(path) {
      requests.push(path);
      await gate;
      return "accepted";
    },
  };
  const first = tagRequest(client, "/tags", accepted);
  const controller = new AbortController();
  const next = tagRequest(client, "/status", accepted, undefined, undefined, controller.signal);
  const cancelled = assert.rejects(next, { name: "AbortError" });
  controller.abort();
  release();
  await Promise.all([first, cancelled]);
  assert.deepEqual(requests, ["/api/v1/file-tags/tags"]);
});
