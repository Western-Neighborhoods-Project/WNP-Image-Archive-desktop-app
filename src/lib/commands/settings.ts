import { invoke } from '@tauri-apps/api/core';

/**
 * Read any setting. Backend requires admin role for secret keys
 * (S3 credentials, API tokens). Use `getPublicSetting` for the
 * non-sensitive UI-facing settings — that won't fail for editor
 * accounts and clearly documents that the key is non-secret.
 */
export async function getSetting(key: string): Promise<string | null> {
  return invoke('get_setting', { key });
}

/**
 * Read a non-secret setting. Available to any logged-in user.
 * Rejects on secret keys (S3 / API tokens) — those go through
 * `getSetting` which requires admin role.
 */
export async function getPublicSetting(key: string): Promise<string | null> {
  return invoke('get_public_setting', { key });
}

export async function setSetting(key: string, value: string): Promise<void> {
  return invoke('set_setting', { key, value });
}

export async function resetCatalog(): Promise<void> {
  return invoke('reset_catalog');
}
