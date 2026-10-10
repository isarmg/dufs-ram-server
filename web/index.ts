import { start } from "./modules/app.ts";
import { startAfterFonts } from "./platform.ts";
import { normalizeWorkspaceLocation } from "./react/navigation.ts";

normalizeWorkspaceLocation();
void startAfterFonts(start);
