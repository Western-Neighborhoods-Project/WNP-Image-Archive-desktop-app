import { invoke } from '@tauri-apps/api/core';

/** Mirrors `models::SourceDirectory` in the Rust crate. */
export interface SourceDirectory {
  id: number;
  path: string;
  label: string;
  createdAt: string;
  imageCount: number;
}

/** One node in the source-directory tree (recursive). */
export interface SourceTreeNode {
  sourceDirectoryId: number;
  label: string;
  /** Empty string at the source root; otherwise forward-slash-joined.
   *  Used as the `relativeDir` filter when this node is selected. */
  relativeDir: string;
  imageCount: number;
  children: SourceTreeNode[];
}

/** Top-level tree entry — pairs a source with its folder hierarchy. */
export interface SourceTreeRoot {
  source: SourceDirectory;
  children: SourceTreeNode[];
}

export async function listSourceDirectories(): Promise<SourceDirectory[]> {
  return invoke('list_source_directories');
}

export async function addSourceDirectory(
  path: string,
  label?: string,
): Promise<SourceDirectory> {
  return invoke('add_source_directory', { path, label });
}

export async function removeSourceDirectory(id: number): Promise<void> {
  return invoke('remove_source_directory', { id });
}

export async function renameSourceDirectory(
  id: number,
  label: string,
): Promise<void> {
  return invoke('rename_source_directory', { id, label });
}

export async function getSourceDirectoryTree(): Promise<SourceTreeRoot[]> {
  return invoke('get_source_directory_tree');
}
