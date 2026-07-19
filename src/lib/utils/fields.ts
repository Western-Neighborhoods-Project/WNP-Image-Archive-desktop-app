// Canonical human labels for the editable image-metadata fields. One source of
// truth for the detail view, the audit-entry rows, and the audit-log filter
// dropdown, so adding or renaming a field can't leave one view showing raw
// snake_case or the filter missing an option.

/** Field keys in display order — drives the audit filter dropdown order. */
export const FIELD_KEYS = [
  'title',
  'description',
  'city',
  'state',
  'country',
  'keywords',
  'date_display',
  'date_start',
  'date_end',
  'photographer',
  'donor',
  'acquisition_date',
  'usage_rights',
  'internal_notes',
] as const;

export const FIELD_LABELS: Record<string, string> = {
  title: 'Title',
  description: 'Description',
  city: 'City',
  state: 'State',
  country: 'Country',
  keywords: 'Keywords',
  date_display: 'Date (display)',
  date_start: 'Date start',
  date_end: 'Date end',
  photographer: 'Photographer',
  donor: 'Donor',
  acquisition_date: 'Acquisition date',
  usage_rights: 'Usage rights',
  internal_notes: 'Internal notes',
};

/** Human label for a field key, falling back to the raw key. */
export function fieldLabel(field: string): string {
  return FIELD_LABELS[field] ?? field;
}

/** {value,label} options for the audit-log field filter, prefixed with "All". */
export const FIELD_FILTER_OPTIONS: { value: string; label: string }[] = [
  { value: 'all', label: 'All fields' },
  ...FIELD_KEYS.map((k) => ({ value: k, label: FIELD_LABELS[k] })),
];
