/**
 * Single source of truth for the keyboard-shortcut reference. Rendered by
 * both the help dialog (`?` → ShortcutsHelp.svelte) and Settings → Keyboard
 * (KeyboardPage.svelte). The actual handlers live in keyboardShortcuts.ts
 * (global ⌘-keys and G-chords, installed from src/routes/+page.svelte) and
 * in view-local listeners (DetailView ⌘⇧S, RequestsView ⌘↵, CommandBar).
 *
 * If you add or remove a shortcut anywhere in the app, update this list —
 * it is the only place the two reference UIs read from.
 */

export interface ShortcutDef {
  keys: string[];
  label: string;
  /** Optional clarifier shown in muted text under the label. */
  hint?: string;
  /** Render `keys` as muted prose instead of keycaps — for pointer
   *  gestures like "drag from empty" that aren't literal keys. */
  prose?: boolean;
}

export interface ShortcutGroup {
  title: string;
  /** Optional clarifier shown under the group title. */
  sub?: string;
  /** Pointer-interaction group: listed on the Settings → Keyboard
   *  reference page but skipped by the compact `?` overlay, which
   *  sticks to actual keyboard shortcuts. */
  pointer?: boolean;
  items: ShortcutDef[];
}

export const SHORTCUT_GROUPS: ShortcutGroup[] = [
  {
    title: 'Global',
    items: [
      { keys: ['⌘', 'K'], label: 'Open command bar', hint: 'Search images, jump to a view, run an action' },
      { keys: ['⌘', ';'], label: 'Open settings' },
      {
        keys: ['⌘', '⇧', 'B'],
        label: 'Report a problem',
        hint: 'Only when debugging is turned on in Settings → Debugging.',
      },
      { keys: ['⌘', '⇧', 'L'], label: 'Log out' },
      { keys: ['?'], label: 'Show keyboard shortcuts' },
      { keys: ['Esc'], label: 'Close dialog or command bar' },
    ],
  },
  {
    title: 'Navigate',
    sub: 'Press G, then the letter — both keystrokes within about a second.',
    items: [
      { keys: ['G', 'A'], label: 'All images' },
      { keys: ['G', 'R'], label: 'Recently viewed' },
      { keys: ['G', 'Q'], label: 'Image requests' },
      { keys: ['G', 'L'], label: 'Audit log' },
      { keys: ['G', 'S'], label: 'Settings', hint: 'Admins only — editors are returned to the library.' },
    ],
  },
  {
    title: 'Inside the command bar',
    items: [
      { keys: ['↑'], label: 'Previous result' },
      { keys: ['↓'], label: 'Next result' },
      { keys: ['↵'], label: 'Open / run selected' },
      { keys: ['⌘', '1–9'], label: 'Jump to N-th result' },
      {
        keys: ['wnp…'],
        label: 'Direct catalog routing',
        hint: 'Type a catalog number like wnp27.4283 to pin that exact image to the top.',
      },
    ],
  },
  {
    title: 'Detail view',
    items: [
      { keys: ['⌘', '⇧', 'S'], label: 'Share image', hint: 'Opens the share dialog for the image being viewed.' },
    ],
  },
  {
    title: 'Requests view',
    items: [
      {
        keys: ['⌘', '↵'],
        label: 'Fulfill selected order',
        hint: 'Only when an order is selected and still processing.',
      },
    ],
  },
  {
    title: 'Library grid',
    pointer: true,
    items: [
      { keys: ['click'], label: 'Open image in detail view', prose: true },
      { keys: ['⌘', 'click'], label: 'Toggle image in selection' },
      { keys: ['⇧', 'click'], label: 'Range select from last clicked' },
      { keys: ['drag from empty'], label: 'Marquee selection', prose: true },
      { keys: ['drag image'], label: 'Drag-and-drop into a sidebar collection', prose: true },
      { keys: ['⌃', 'click'], label: 'Open context menu' },
    ],
  },
];
