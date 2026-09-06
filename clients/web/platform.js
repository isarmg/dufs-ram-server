export { createAdministratorApiClient, isAdministratorPassword } from "@sarmg/admin-web";
export { isAdministratorSession, isAdministratorLoginRequest, isErrorEnvelope } from "@sarmg/contracts";
export { ApiClientError } from "@sarmg/http-client";
import { mountFileWorkspace as renderFiles, mountLoginPage as renderLogin } from "./react/application.js";
/** @param {HTMLElement} container */
export function mountFileWorkspace(container) { renderFiles(container); }
/** @param {HTMLElement} container
 * @param {(username: string, password: string) => Promise<void>} login
 * @param {(error: unknown) => string} errorMessage */
export function mountLoginPage(container, login, errorMessage) { renderLogin(container, login, errorMessage); }
export { t, getLocale, initializeLanguage, switchLanguage, languageLabel, createLanguageControl, validationMessage } from "@sarmg/admin-ui/i18n";
import "@sarmg/design-tokens/tokens.css";
import "@sarmg/design-tokens/tokens.dark.css";
import "@sarmg/design-tokens/reset.css";
import "@sarmg/design-tokens/accessibility.css";
import "@sarmg/web-fonts/fonts.css";
import "@sarmg/admin-ui/styles.css";
export { default as fontLicenseUrl } from "@sarmg/web-fonts/OFL.txt?url";
export { default as cjkFontLicenseUrl } from "@sarmg/web-fonts/CJK-LICENSE.txt?url";
