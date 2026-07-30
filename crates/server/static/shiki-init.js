// Shiki is loaded only on pages that contain a fenced code block with a
// language annotation. Keeping this separate avoids a highlighting cost for
// prose-only pages and preserves Mermaid's dedicated renderer.
// Shiki's bundled theme is named `tokyo-night`; Storm is provided here as a
// compatible TextMate theme so course blocks use the requested variant.
const TOKYONIGHT_STORM = {
  name: 'tokyonight-storm',
  type: 'dark',
  colors: {
    'editor.background': '#24283b',
    'editor.foreground': '#c0caf5',
  },
  settings: [
    { settings: { background: '#24283b', foreground: '#c0caf5' } },
    { scope: ['comment', 'punctuation.definition.comment'], settings: { foreground: '#565f89' } },
    { scope: ['string', 'constant.other.symbol'], settings: { foreground: '#9ece6a' } },
    { scope: ['constant.numeric', 'constant.language'], settings: { foreground: '#ff9e64' } },
    { scope: ['keyword', 'storage', 'support.type'], settings: { foreground: '#bb9af7' } },
    { scope: ['entity.name.function', 'support.function'], settings: { foreground: '#7aa2f7' } },
    { scope: ['variable', 'entity.name.tag', 'entity.other.attribute-name'], settings: { foreground: '#7dcfff' } },
    { scope: ['punctuation', 'meta.brace'], settings: { foreground: '#a9b1d6' } },
  ],
};

(async function highlightCodeBlocks() {
  const blocks = Array.from(
    document.querySelectorAll('pre:not(.mermaid) > code[class*="language-"]'),
  );
  if (blocks.length === 0) return;

  try {
    // Pin the CDN dependency: Shiki is ESM-only and its generated markup
    // includes token colours, so no grammar-specific CSS is needed here.
    const { codeToHtml } = await import('https://esm.sh/shiki@3.0.0');

    await Promise.all(blocks.map(async (code) => {
      const languageClass = Array.from(code.classList)
        .find((name) => name.startsWith('language-'));
      if (!languageClass) return;

      const pre = code.parentElement;
      if (!pre) return;

      const html = await codeToHtml(code.textContent || '', {
        lang: languageClass.slice('language-'.length),
        theme: TOKYONIGHT_STORM,
      });
      const template = document.createElement('template');
      template.innerHTML = html.trim();
      const highlighted = template.content.firstElementChild;
      if (!highlighted) return;

      highlighted.classList.add('cb-code-block');
      pre.replaceWith(highlighted);
    }));
  } catch (error) {
    // A failed grammar or an unavailable CDN leaves the original, readable
    // code block in place. Highlighting must never make course content fail.
    console.warn('No se pudo resaltar un bloque de código con Shiki.', error);
  }
}());
