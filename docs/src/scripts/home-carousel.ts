const initialized = new WeakSet<HTMLElement>();

/** Native scrolling is the baseline; controls never hide any slide. */
export function enhanceCarousel(root: HTMLElement) {
  const track = root.querySelector<HTMLElement>('[data-carousel-track]');
  const slides = Array.from(root.querySelectorAll<HTMLElement>('[data-home-slide]'));
  const previous = root.querySelector<HTMLButtonElement>('[data-carousel-previous]');
  const next = root.querySelector<HTMLButtonElement>('[data-carousel-next]');
  const position = root.querySelector<HTMLOutputElement>('[data-carousel-position]');
  const controls = root.querySelector<HTMLElement>('[data-carousel-controls]');
  if (!track || !slides.length || !previous || !next || !position || !controls || initialized.has(root)) return;
  initialized.add(root);
  const abort = new AbortController();
  const options = { signal: abort.signal };
  let current = 0;
  let frame = 0;
  let destination: number | null = null;
  const offset = (index: number) => slides[index].offsetLeft - slides[0].offsetLeft;
  const render = () => {
    previous.disabled = current === 0;
    next.disabled = current === slides.length - 1;
    position.textContent = `${current + 1} / ${slides.length}`;
  };
  const update = () => {
    frame = 0;
    if (destination !== null) {
      if (Math.abs(track.scrollLeft - offset(destination)) > 2) return;
      destination = null;
    }
    let nearest = 0;
    slides.forEach((_, index) => {
      if (Math.abs(track.scrollLeft - offset(index)) < Math.abs(track.scrollLeft - offset(nearest))) nearest = index;
    });
    if (nearest !== current) { current = nearest; render(); }
  };
  const go = (index: number, focus = false, instant = false) => {
    current = Math.max(0, Math.min(slides.length - 1, index));
    destination = current;
    render();
    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    track.scrollTo({ left: offset(current), behavior: reduced || instant ? 'instant' : 'smooth' });
    if (focus) slides[current].focus({ preventScroll: true });
  };
  previous.addEventListener('click', () => go(current - 1), options);
  next.addEventListener('click', () => go(current + 1), options);
  root.addEventListener('keydown', (event) => {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
    if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
    const target = event.target instanceof Element ? event.target : null;
    // A focused code scroller keeps its own horizontal keyboard navigation.
    if (target?.closest('pre, figure [role="region"], input, textarea, select, [contenteditable="true"]')) return;
    event.preventDefault();
    go(current + (event.key === 'ArrowRight' ? 1 : -1), Boolean(target?.closest('[data-home-slide]')));
  }, options);
  track.addEventListener('scroll', () => {
    if (!frame) frame = window.requestAnimationFrame(update);
  }, { ...options, passive: true });
  track.addEventListener('pointerdown', () => { destination = null; }, options);
  track.addEventListener('wheel', () => { destination = null; }, { ...options, passive: true });
  track.addEventListener('scrollend', () => { destination = null; update(); }, options);
  const resize = new ResizeObserver(() => go(current, false, true));
  resize.observe(track);
  const cleanup = () => {
    abort.abort();
    resize.disconnect();
    if (frame) window.cancelAnimationFrame(frame);
    initialized.delete(root);
  };
  document.addEventListener('astro:before-swap', cleanup, { once: true, signal: abort.signal });
  controls.hidden = false;
  update();
  render();
  return cleanup;
}

export function initializeCarousels() {
  document.querySelectorAll<HTMLElement>('[data-home-carousel]').forEach(enhanceCarousel);
}
