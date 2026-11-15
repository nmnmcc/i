// @fumadocs/astro 0.3.0 restores hidden code nodes without their data.meta.
// Carry metadata through hProperties, which its restore step preserves and
// Fumadocs' code highlighter reads as a fallback to data.meta.
export const fenceMetadata = {
  name: 'i-fence-metadata',
  code(node, ctx) {
    if (!node.meta) return;
    ctx.setProperty(node, 'data', {
      ...node.data,
      hProperties: { ...node.data?.hProperties, metastring: node.meta },
    });
  },
};
