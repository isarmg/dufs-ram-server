import { createSarmgReactViteConfig } from "@sarmg/web-toolchain/vite";
import { createHash } from "node:crypto";

const config = createSarmgReactViteConfig({ base: "./" });
config.build = {
  ...config.build, outDir: "clients/web/dist", assetsInlineLimit: 0, cssCodeSplit: false,
  rollupOptions: {
    input: "clients/web/platform.js", preserveEntrySignatures: "strict",
    output: { format: "es", entryFileNames: "platform.js", chunkFileNames: "[hash].js",
      assetFileNames: asset => asset.names.some(name => name.endsWith(".css")) ? "platform.css" : "[name][extname]" },
  },
};
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
