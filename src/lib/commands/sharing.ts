import { invoke } from '@tauri-apps/api/core';

/** Mirrors `models::CreateShareLinkResult` (camelCase via serde). */
export interface CreateShareLinkResult {
  imageUrl: string;
  recipientEmail: string;
  resolutionLabel: string;
}

/** Resize → upload to B2 → POST to OpenSFHistory which sends the email
 *  via Postmark. Returns the final image URL on success. */
export async function createShareLink(args: {
  imageId: number;
  recipientEmail: string;
  resolution: 'low' | 'high' | 'full';
  purpose: string;
}): Promise<CreateShareLinkResult> {
  return invoke('create_share_link', args);
}
