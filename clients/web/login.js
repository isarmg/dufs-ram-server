import { t, validationMessage } from "./dist/platform.js";
import { localizeStaticPage } from "./modules/language.js";
localizeStaticPage(true);
import { isAdministratorLoginRequest, isAdministratorPassword } from "./dist/platform.js";
import { administratorApi, authenticationErrorMessage } from "./modules/platform-session.js";

const form = /** @type {HTMLFormElement} */ (document.querySelector(".login-card"));
const username = /** @type {HTMLInputElement} */ (document.querySelector("#username"));
const password = /** @type {HTMLInputElement} */ (document.querySelector("#password"));
const submit = /** @type {HTMLButtonElement} */ (document.querySelector('button[type="submit"]'));
const errorRow = /** @type {HTMLElement} */ (document.querySelector(".error-row"));
const errorText = /** @type {HTMLElement} */ (document.querySelector(".login-error"));
let pending = false;
for (const input of [username, password]) {
  input.addEventListener("invalid", event => event.preventDefault());
  input.addEventListener("input", () => setError(""));
}

form.addEventListener("submit", event => {
  event.preventDefault();
  if (pending) return;
  for (const input of [username, password]) {
    if (!input.validity.valid) {
      const message = input.validity.valueMissing
        ? input === username ? t("请输入用户名。", "Enter your username.") : t("请输入密码。", "Enter your password.")
        : validationMessage(input);
      setError(message, input);
      input.focus();
      return;
    }
  }
  if (!isAdministratorLoginRequest({ username: username.value, password: password.value }) || !isAdministratorPassword(password.value)) {
    password.value = "";
    setError(t("请输入有效的管理员用户名和密码。", "Enter a valid administrator username and password."));
    password.focus();
    return;
  }
  void login();
});

async function login() {
  setError("");
  pending = true;
  submit.disabled = true;
  username.readOnly = true;
  password.readOnly = true;
  form.setAttribute("aria-busy", "true");
  let failed = false;
  try {
    await administratorApi.login(username.value, password.value);
    location.replace("/");
  } catch (error) {
    failed = true;
    setError(authenticationErrorMessage(error));
  } finally {
    password.value = "";
    pending = false;
    submit.disabled = false;
    username.readOnly = false;
    password.readOnly = false;
    form.removeAttribute("aria-busy");
    if (failed) password.focus();
  }
}

/** @param {string} message @param {HTMLInputElement} [invalidInput] */
function setError(message, invalidInput) {
  errorText.textContent = message;
  errorRow.classList.toggle("hidden", message.length === 0);
  for (const input of [username, password]) {
    if (input === invalidInput) input.setAttribute("aria-invalid", "true");
    else input.removeAttribute("aria-invalid");
    if (message) input.setAttribute("aria-describedby", "login-error");
    else input.removeAttribute("aria-describedby");
  }
}
