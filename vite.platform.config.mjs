import { createSarmgNativeModuleViteConfig } from "@sarmg/web-toolchain/native";
import { createHash } from "node:crypto";

const config = createSarmgNativeModuleViteConfig({ entry: "clients/web/platform.js", outDir: "clients/web/dist" });
// Foundation's authored CSS masks are compile-time inputs, not user images.
// Emit them as immutable same-origin assets so Dufs keeps data: disallowed.
config.plugins.push({
  name: "dufs-same-origin-foundation-icons",
  enforce: "post",
  generateBundle: { order: "post", handler(_options, bundle) {
    for (const asset of Object.values(bundle)) {
      if (asset.type !== "asset" || !asset.fileName.endsWith(".css")) continue;
      asset.source = String(asset.source).replace(
        /url\("data:image\/svg\+xml,([^"\n]+)"\)/gu,
        (_match, encoded) => {
          const source = decodeURIComponent(encoded);
          const hash = createHash("sha256").update(source).digest("hex");
          const fileName = `foundation-icon-${hash}.svg`;
          this.emitFile({ type: "asset", fileName, source });
          return `url("./${fileName}")`;
        },
      );
    }
  } },
});
export default config;
