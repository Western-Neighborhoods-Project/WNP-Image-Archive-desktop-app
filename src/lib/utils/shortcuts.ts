/**
 * Single source of truth for the keyboard shortcuts surfaced in the help
 * dialog (`?`) and the sidebar/status-bar `kbd` hints. The actual handlers
 * live in keyboardShortcuts.ts; this file is the metadata used to render
 * "what does ⌘K do?" in the help modal.
 *
 * Keep these in sync with the chord/cmdKey table installed in
 * src/routes/+page.svelte. If you add a shortcut, add it here too.
 */

export interface ShortcutDef {
  keys: string[];
  label: string;
  /** Optional clarifier shown in muted text under the label. */
  hint?: string;
}

export interface ShortcutGroup {
  title: string;
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
      { keys: ['?'], label: 'Show keyboard shortcuts' },
      { keys: ['Esc'], label: 'Close dialog or command bar' },
    ],
  },
  {
    title: 'Navigate',
    items: [
      { keys: ['G', 'A'], label: 'All images' },
      { keys: ['G', 'R'], label: 'Recently viewed' },
      { keys: ['G', 'Q'], label: 'Pending requests' },
      { keys: ['G', 'L'], label: 'Audit log' },
      { keys: ['G', 'S'], label: 'Settings' },
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
    title: 'Requests view',
    items: [
      {
        keys: ['⌘', '↵'],
        label: 'Fulfill selected order',
        hint: 'Only when an order is selected and still processing.',
      },
    ],
  },
];
