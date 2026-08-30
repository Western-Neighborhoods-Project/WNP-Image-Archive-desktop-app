import { invoke } from '@tauri-apps/api/core';

/** Mirrors `ReportCategory` in src-tauri/src/bug_reports.rs. */
export type ReportCategory = 'bug' | 'feature' | 'idea' | 'data' | 'ux';

export interface SubmittedIssue {
  number: number;
  htmlUrl: string;
}

/**
 * File a report as a GitHub issue. Backend requires a logged-in session,
 * the debugging toggle on, and a configured token; it appends app version /
 * OS / view / username context to the issue body.
 */
export async function submitBugReport(
  category: ReportCategory,
  description: string,
  currentView: string,
): Promise<SubmittedIssue> {
  return invoke('submit_bug_report', { category, description, currentView });
}
