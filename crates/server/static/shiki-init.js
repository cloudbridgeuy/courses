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

// Pin the CDN dependency: Shiki is ESM-only and its generated markup includes
// token colours, so no grammar-specific CSS is needed here. The import promise
// is cached so the static pass and on-demand callers share one load.
let cbShikiModule = null;
function cbLoadShiki() {
  if (!cbShikiModule) cbShikiModule = import('https://esm.sh/shiki@3.0.0');
  return cbShikiModule;
}

// On-demand highlighter for content that appears after the static pass below
// (e.g. the <cb-http> response panel). Returns the highlighted <pre> element,
// or null when the grammar or the CDN fails — callers keep their plain text.
window.cbShiki = {
  highlight: async function (code, lang) {
    try {
      const { codeToHtml } = await cbLoadShiki();
      const html = await codeToHtml(code, { lang, theme: TOKYONIGHT_STORM });
      const template = document.createElement('template');
      template.innerHTML = html.trim();
      return template.content.firstElementChild;
    } catch (error) {
      console.warn('No se pudo resaltar un bloque de código con Shiki.', error);
      return null;
    }
  },
};

// A fence's `code` element can hold content-variable token spans
// (`<span class="cb-var" data-var="name">`) alongside plain text nodes.
// `codeToHtml` only ever sees the flattened string (`code.textContent`), so
// rebuilding the block would otherwise drop those spans. This walks the same
// child nodes in the same order and records each token's `{start, end}`
// character range in that flattened string, so shiki can be asked to paint
// the identical `class`/`data-var` markup back onto the ranges it rebuilds.
function cbVarDecorations(code) {
  const decorations = [];
  let offset = 0;
  for (const node of code.childNodes) {
    const text = node.textContent || '';
    if (node.nodeType === Node.ELEMENT_NODE && node.classList.contains('cb-var')) {
      const name = node.dataset.var;
      if (name) {
        decorations.push({
          start: offset,
          end: offset + text.length,
          properties: { class: 'cb-var', 'data-var': name },
        });
      }
    }
    offset += text.length;
  }
  return decorations;
}

(async function highlightCodeBlocks() {
  const blocks = Array.from(
    document.querySelectorAll('pre:not(.mermaid) > code[class*="language-"]'),
  );
  if (blocks.length === 0) return;

  try {
    const { codeToHtml } = await cbLoadShiki();

    await Promise.all(blocks.map(async (code) => {
      const languageClass = Array.from(code.classList)
        .find((name) => name.startsWith('language-'));
      if (!languageClass) return;

      const pre = code.parentElement;
      if (!pre) return;

      const html = await codeToHtml(code.textContent || '', {
        lang: languageClass.slice('language-'.length),
        theme: TOKYONIGHT_STORM,
        decorations: cbVarDecorations(code),
      });
      const template = document.createElement('template');
      template.innerHTML = html.trim();
      const highlighted = template.content.firstElementChild;
      if (!highlighted) return;

      highlighted.classList.add('cb-code-block');
      pre.replaceWith(highlighted);
      // The rebuilt block carries fresh (unfilled) token spans; apply the
      // page's stored values to them right away. `vars.js` only runs on
      // pages that actually declare a token, so this is undefined elsewhere.
      window.cbVars?.apply?.();
    }));
  } catch (error) {
    // A failed grammar or an unavailable CDN leaves the original, readable
    // code block in place. Highlighting must never make course content fail.
    console.warn('No se pudo resaltar un bloque de código con Shiki.', error);
  }
}());
