import assert from 'node:assert/strict';
import test from 'node:test';
import { enhanceCarousel } from '../src/scripts/home-carousel.ts';

class ElementFixture {
  listeners = new Map();
  hidden = true;
  disabled = false;
  scrollLeft = 0;
  requests = [];
  textContent = '';
  constructor(kind = '') { this.kind = kind; }
  addEventListener(type, listener, options = {}) {
    const entry = { listener, once: options.once };
    const entries = this.listeners.get(type) ?? [];
    entries.push(entry);
    this.listeners.set(type, entries);
    options.signal?.addEventListener('abort', () => {
      this.listeners.set(type, (this.listeners.get(type) ?? []).filter((item) => item !== entry));
    }, { once: true });
  }
  emit(type, values = {}) {
    const event = { target: this, preventDefault() { this.prevented = true; }, ...values };
    for (const entry of [...(this.listeners.get(type) ?? [])]) {
      entry.listener(event);
      if (entry.once) this.listeners.set(type, this.listeners.get(type).filter((item) => item !== entry));
    }
    return event;
  }
  closest(selector) {
    if (selector === '[data-home-slide]') return this.kind === 'slide' ? this : null;
    return this.kind === 'code' ? this : null;
  }
  focus(options) { this.focused = options; }
  scrollTo(options) { this.requests.push(options); }
}

function fixture({ reduced = false } = {}) {
  const root = new ElementFixture();
  const track = new ElementFixture();
  const previous = new ElementFixture();
  const next = new ElementFixture();
  const position = new ElementFixture();
  const controls = new ElementFixture();
  const slides = [0, 1, 2].map((index) => Object.assign(new ElementFixture('slide'), { offsetLeft: 20 + index * 1000 }));
  const selectors = new Map([
    ['[data-carousel-track]', track], ['[data-carousel-previous]', previous],
    ['[data-carousel-next]', next], ['[data-carousel-position]', position],
    ['[data-carousel-controls]', controls],
  ]);
  root.querySelector = (selector) => selectors.get(selector);
  root.querySelectorAll = () => slides;
  const document = new ElementFixture();
  const callbacks = new Map();
  let id = 0;
  const window = {
    matchMedia: () => ({ matches: reduced }),
    requestAnimationFrame(callback) { callbacks.set(++id, callback); return id; },
    cancelAnimationFrame(key) { callbacks.delete(key); },
  };
  class ResizeFixture {
    constructor(callback) { this.callback = callback; ResizeFixture.last = this; }
    observe(target) { this.target = target; }
    disconnect() { this.disconnected = true; }
  }
  Object.assign(globalThis, { Element: ElementFixture, window, document, ResizeObserver: ResizeFixture });
  const flush = () => { const list = [...callbacks.values()]; callbacks.clear(); list.forEach((callback) => callback()); };
  return { root, track, previous, next, position, controls, slides, flush, document, ResizeFixture };
}

test('controls enhance the native list once, with correct end states', () => {
  const f = fixture();
  const cleanup = enhanceCarousel(f.root);
  assert.equal(f.controls.hidden, false);
  assert.equal(f.previous.disabled, true);
  assert.equal(f.next.disabled, false);
  assert.equal(f.position.textContent, '1 / 3');
  assert.equal(enhanceCarousel(f.root), undefined);
  cleanup();
});

test('rapid next clicks retain their destination during smooth scrolling', () => {
  const f = fixture();
  const cleanup = enhanceCarousel(f.root);
  f.next.emit('click');
  assert.deepEqual(f.track.requests.at(-1), { left: 1000, behavior: 'smooth' });
  f.next.emit('click');
  assert.deepEqual(f.track.requests.at(-1), { left: 2000, behavior: 'smooth' });
  f.track.scrollLeft = 200;
  f.track.emit('scroll'); f.flush();
  assert.equal(f.position.textContent, '3 / 3');
  f.track.scrollLeft = 2000;
  f.track.emit('scroll'); f.flush();
  assert.equal(f.next.disabled, true);
  cleanup();
});

test('arrow keys move and focus slides; code and modified arrows are left alone', () => {
  const f = fixture();
  const cleanup = enhanceCarousel(f.root);
  const moved = f.root.emit('keydown', { key: 'ArrowRight', target: f.slides[0] });
  assert.equal(moved.prevented, true);
  assert.deepEqual(f.slides[1].focused, { preventScroll: true });
  const count = f.track.requests.length;
  const code = f.root.emit('keydown', { key: 'ArrowRight', target: new ElementFixture('code') });
  const modified = f.root.emit('keydown', { key: 'ArrowRight', ctrlKey: true });
  assert.equal(code.prevented, undefined);
  assert.equal(modified.prevented, undefined);
  assert.equal(f.track.requests.length, count);
  cleanup();
});

test('native swipe or wheel scrolling updates the indicator', () => {
  const f = fixture();
  const cleanup = enhanceCarousel(f.root);
  f.next.emit('click');
  f.track.emit('pointerdown');
  f.track.scrollLeft = 1950;
  f.track.emit('scroll'); f.flush();
  assert.equal(f.position.textContent, '3 / 3');
  assert.equal(f.next.disabled, true);
  cleanup();
});

test('reduced motion and resize use exact, instant snap offsets', () => {
  const f = fixture({ reduced: true });
  const cleanup = enhanceCarousel(f.root);
  f.next.emit('click');
  assert.deepEqual(f.track.requests.at(-1), { left: 1000, behavior: 'instant' });
  f.slides.forEach((slide, index) => { slide.offsetLeft = 20 + index * 1200; });
  f.ResizeFixture.last.callback();
  assert.deepEqual(f.track.requests.at(-1), { left: 1200, behavior: 'instant' });
  cleanup();
});

test('route swaps remove observers and listeners before the next visit', () => {
  const f = fixture();
  enhanceCarousel(f.root);
  f.document.emit('astro:before-swap');
  f.next.emit('click');
  assert.equal(f.track.requests.length, 0);
  assert.equal(f.ResizeFixture.last.disconnected, true);
  const cleanup = enhanceCarousel(f.root);
  assert.equal(typeof cleanup, 'function');
  cleanup();
});
