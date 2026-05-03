import { invoke } from '@tauri-apps/api/core';

export type UserRole = 'admin' | 'editor';

export interface UserSession {
  userId: number;
  username: string;
  role: UserRole;
  loginAtMs: number;
}

/** True when no users exist yet — frontend should show "Create your first
 *  admin" form instead of the login screen. */
export async function isSetupRequired(): Promise<boolean> {
  return invoke('is_setup_required');
}

export async function createFirstAdmin(
  username: string,
  password: string,
): Promise<UserSession> {
  return invoke('create_first_admin', { username, password });
}

export async function login(
  username: string,
  password: string,
): Promise<UserSession> {
  return invoke('login', { username, password });
}

export async function logout(): Promise<void> {
  return invoke('logout');
}

export async function getCurrentUser(): Promise<UserSession | null> {
  return invoke('get_current_user');
}
