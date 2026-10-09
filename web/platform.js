export { createAdministratorApiClient, isAdministratorPassword } from "@xcss/admin-web";
export { isAdministratorSession, isAdministratorLoginRequest, isErrorEnvelope } from "@xcss/contracts";
export { ApiClientError } from "@xcss/http-client";
import { startAfterFonts as startPrepared } from "@xcss/web-fonts";
/** @param {() => void} start */
export function startAfterFonts(start) { return startPrepared(start); }
import { mountFileWorkspace as renderFiles, mountLoginPage as renderLogin } from "./react/application.js";
/** @param {HTMLElement} container
 * @param {import("@xcss/admin-web").AdministratorApiClient} client */
export function mountFileWorkspace(container, client) { renderFiles(container, client); }
/** @param {HTMLElement} container
 * @param {(username: string, password: string) => Promise<void>} login
 * @param {(error: unknown) => string} errorMessage */
export function mountLoginPage(container, login, errorMessage) { renderLogin(container, login, errorMessage); }
export { t, getLocale, initializeLanguage, switchLanguage, languageLabel, createLanguageControl, validationMessage } from "@xcss/admin-ui/i18n";
import "@xcss/design-tokens/tokens.css";
import "@xcss/design-tokens/tokens.dark.css";
import "@xcss/design-tokens/reset.css";
import "@xcss/design-tokens/accessibility.css";
import "@xcss/web-fonts/fonts.css";
import "@xcss/admin-ui/styles.css";
export { default as fontLicenseUrl } from "@xcss/web-fonts/OFL.txt?url";
export { default as cjkFontLicenseUrl } from "@xcss/web-fonts/CJK-LICENSE.txt?url";
