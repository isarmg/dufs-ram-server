import { t } from "../dist/platform.js";
import { createAdministratorApiClient, ApiClientError } from "../dist/platform.js";

// One in-memory platform client per document. No credential storage in HTML or storage APIs.
export const administratorApi = createAdministratorApiClient();

/** @param {unknown} error @returns {string} */
export function authenticationErrorMessage(error) {
  const requestId = error instanceof ApiClientError ? error.requestId : undefined;
  return t("无法完成认证请求，请重试。", "Authentication request could not be completed. Please try again.") +
    (requestId ? t(" 请求标识：{0}", " Request ID: {0}", [requestId]) : "");
}
