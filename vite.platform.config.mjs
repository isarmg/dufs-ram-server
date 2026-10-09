import { readFileSync } from "node:fs";
import { createXcssReactViteConfig } from "@xcss/web-toolchain/vite";
import { createHash } from "node:crypto";

const config = createXcssReactViteConfig({ base: "./" });
// A blocking same-origin stylesheet hides the body before first paint under CSP.
config.plugins.push({
  name: "xczs-font-startup-style",
  generateBundle() {
    this.emitFile({
      type: "asset", fileName: "boot.css",
      source: readFileSync(new URL(import.meta.resolve("@xcss/web-fonts/boot.css"))),
    });
  },
});
config.build = {
  ...config.build, outDir: "web/dist", assetsInlineLimit: 0, cssCodeSplit: false,
  rollupOptions: {
    input: { platform: "web/platform.js", tags: "web/react/tags.jsx" }, preserveEntrySignatures: "strict",
    output: { format: "es", entryFileNames: "[name].js", chunkFileNames: "[hash].js",
      assetFileNames: asset => asset.names.some(name => name.endsWith(".css")) ? "platform.css" : "[name][extname]" },
  },
};
// Foundation's authored CSS masks are compile-time inputs, not user images.
// Emit them as immutable same-origin assets so Xczs keeps data: disallowed.
config.plugins.push({
  name: "xczs-same-origin-foundation-icons",
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
