import { defineConfig, presetIcons, presetUno, presetWebFonts, transformerVariantGroup } from 'unocss'

export default defineConfig({
  presets: [
    presetUno(),
    presetIcons(),
    presetWebFonts({
      fonts: {
        sans: 'IBM Plex Sans',
        display: 'Fraunces',
      },
    }),
  ],
  transformers: [transformerVariantGroup()],
})
