import { t, initializeLanguage, createLanguageControl } from "../dist/platform.js";

/** Localize only explicit authored markers, never API data or arbitrary DOM. */
export function localizeStaticPage(login = false) {
  initializeLanguage();
  for (const element of document.querySelectorAll("[data-i18n-zh]")) {
    element.textContent = t(element.getAttribute("data-i18n-zh") || "", element.textContent || "");
  }
  for (const attribute of ["aria-label", "title"]) {
    for (const element of document.querySelectorAll(`[data-i18n-${attribute}-zh]`)) {
      element.setAttribute(attribute, t(element.getAttribute(`data-i18n-${attribute}-zh`) || "", element.getAttribute(attribute) || ""));
    }
  }
  if (login) {
    document.title = t("登录", "Sign in");
    document.querySelector(".card-actions")?.append(createLanguageControl());
  }
}
