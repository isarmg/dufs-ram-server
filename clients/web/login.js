import { mountLoginPage } from "./dist/platform.js";
import { administratorApi, authenticationErrorMessage } from "./modules/platform-session.js";

const container = document.getElementById("dufs-root");
if (!container) throw new Error("Dufs application root is missing");
mountLoginPage(container, async (username, password) => {
  await administratorApi.login(username, password);
  location.replace("/");
}, authenticationErrorMessage);
