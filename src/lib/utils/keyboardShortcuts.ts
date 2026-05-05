/**
 * Global keyboard shortcuts:
 *   ⌘K        → open command bar (always; built into this util)
 *   ⌘<key>    → caller-supplied cmd-key shortcuts (e.g. ⌘; → settings)
 *   G + <X>   → multi-key chord (Raycast / Vim / GitHub / Linear style),
 *               1s timeout, suppressed while typing in editable elements
 *
 * Single window-level handler installed once at app boot via
 * `installShortcuts()`. Returns a cleanup function for symmetry.
 *
 * Why a leader (`G`) for navigation chords:
 * - frees the bare-letter namespace for view-specific actions
 *   (e.g. `E` to edit, `S` to share inside detail view)
 * - clearer intent — `G then Q` reads as "go to queue"
 * - matches Vim / GitHub / Linear muscle memory
 *
 * Suppression: chord and bare-letter handling never fire while focus is
 * in an input / textarea / contenteditable. ⌘<key> shortcuts always
 * fire (they're explicit modifier presses; user means business).
 *
 * Chord-vs-command-bar: if the command bar is open, chord state is
 * cleared and bare-letter handling skips — the bar's own onkeydown
 * owns arrow keys, Enter, Esc.
 */

import { get } from 'svelte/store';
import { openCommandBar, commandBarOpen } from '$lib/stores/commandBar';
import { toggleShortcutsHelp, shortcutsHelpOpen } from '$lib/stores/shortcutsHelp';

export type ShortcutAction = () => void;

interface ShortcutsConfig {
  /** Single-letter actions that fire after the `G` leader. Keys are
   *  lowercased letters; `g a` → chords['a']. */
  chords: Record<string, ShortcutAction>;
  /** Modifier-key shortcuts. Keys are the lowercased event.key value
   *  (e.g. ';' for ⌘;, ',' for ⌘,). ⌘K is reserved for the command bar
   *  and should not be supplied here. */
  cmdKey?: Record<string, ShortcutAction>;
  /** ⌘⇧+key shortcuts (e.g. 'l' for ⌘⇧L → log out). Looked up only
   *  when shift is held alongside cmd/ctrl. */
  cmdShiftKey?: Record<string, ShortcutAction>;
}

const CHORD_TIMEOUT_MS = 1000;

let chordPending = false;
let chordTimer: ReturnType<typeof setTimeout> | null = null;
let config: ShortcutsConfig = { chords: {}, cmdKey: {}, cmdShiftKey: {} };

/** True when the event target is a form field the user is typing into.
 *  Exported so view-specific shortcuts (e.g. ⌘↵ in RequestsView) share
 *  the same suppression rule. */
export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;
  if (target.isContentEditable) return true;
  return false;
}

function clearChord() {
  chordPending = false;
  if (chordTimer !== null) {
    clearTimeout(chordTimer);
    chordTimer = null;
  }
}

function startChord() {
  chordPending = true;
  chordTimer = setTimeout(clearChord, CHORD_TIMEOUT_MS);
}

function handleKeyDown(e: KeyboardEvent) {
  const key = e.key.toLowerCase();
  const cmdLike = e.metaKey || e.ctrlKey;

  // ── Modifier-key shortcuts ────────────────────────────────────────
  if (cmdLike && !e.altKey) {
    // ⌘K → command bar (always available, even from inside form fields).
    // Only fires without Shift; ⌘⇧K stays free for callers.
    if (key === 'k' && !e.shiftKey) {
      e.preventDefault();
      openCommandBar();
      clearChord();
      return;
    }
    // ⌘⇧<key> takes precedence when Shift is held; otherwise plain ⌘<key>.
    const lookup = e.shiftKey ? config.cmdShiftKey : config.cmdKey;
    const cmdAction = lookup?.[key];
    if (cmdAction) {
      e.preventDefault();
      cmdAction();
      clearChord();
      return;
    }
  }

  // ── Bare-letter / chord handling ──────────────────────────────────
  // Don't process while typing or while the command bar is open.
  if (isEditableTarget(e.target)) {
    clearChord();
    return;
  }
  if (get(commandBarOpen)) return;

  // `?` toggles the keyboard-shortcuts help. Allowed even with Shift
  // (since Shift+/ is what produces `?` on a US keyboard) but blocked
  // with the cmd-style modifiers.
  if (e.key === '?' && !e.metaKey && !e.ctrlKey && !e.altKey) {
    e.preventDefault();
    toggleShortcutsHelp();
    clearChord();
    return;
  }

  // The shortcuts help dialog handles its own Esc; don't double-process
  // chords while it's open.
  if (get(shortcutsHelpOpen)) return;

  // Ignore when modifier keys are held — only bare keypresses chord.
  if (e.metaKey || e.ctrlKey || e.altKey) return;

  if (chordPending) {
    const action = config.chords[key];
    if (action) {
      e.preventDefault();
      action();
    }
    clearChord();
    return;
  }

  if (key === 'g') {
    e.preventDefault();
    startChord();
  }
}

/**
 * Install the global keyboard shortcut handler. Returns a teardown fn.
 *
 * Example:
 *   installShortcuts({
 *     chords: { a: goToLibrary, q: goToRequests, s: goToSettings },
 *     cmdKey: { ';': goToSettings },  // ⌘; also opens settings
 *   });
 */
export function installShortcuts(opts: ShortcutsConfig): () => void {
  config = {
    chords: { ...opts.chords },
    cmdKey: { ...(opts.cmdKey ?? {}) },
    cmdShiftKey: { ...(opts.cmdShiftKey ?? {}) },
  };
  window.addEventListener('keydown', handleKeyDown);
  return () => {
    window.removeEventListener('keydown', handleKeyDown);
    clearChord();
    config = { chords: {}, cmdKey: {}, cmdShiftKey: {} };
  };
}
