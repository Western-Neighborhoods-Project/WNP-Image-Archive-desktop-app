import { invoke } from '@tauri-apps/api/core';
import type { UserRole } from './auth';

/** Mirrors `auth::User` (no password hash). */
export interface User {
  id: number;
  username: string;
  role: UserRole;
  createdAt: string;
  lastLoginAt: string | null;
}

export async function listUsers(): Promise<User[]> {
  return invoke('list_users');
}

export async function createUser(
  username: string,
  password: string,
  role: UserRole,
): Promise<User> {
  return invoke('create_user', { username, password, role });
}

export async function updateUserRole(
  userId: number,
  role: UserRole,
): Promise<void> {
  return invoke('update_user_role', { userId, role });
}

export async function updateUserPassword(
  userId: number,
  newPassword: string,
  currentPassword?: string,
): Promise<void> {
  // currentPassword is required by the backend when changing your OWN password
  // and ignored when an admin resets another user's.
  return invoke('update_user_password', {
    userId,
    newPassword,
    currentPassword: currentPassword ?? null,
  });
}

export async function deleteUser(userId: number): Promise<void> {
  return invoke('delete_user', { userId });
}
