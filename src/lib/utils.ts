import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import type { Component } from "svelte";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

// Type helpers expected by shadcn-svelte components
export type WithElementRef<T, E extends Element = HTMLElement> = T & {
  ref?: E | null;
};

export type WithoutChild<T> = Omit<T, "child">;
export type WithoutChildren<T> = Omit<T, "children">;
export type WithoutChildrenOrChild<T> = Omit<T, "children" | "child">;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithChild<T, U extends Record<string, any> = Record<string, never>> = Omit<T, "child" | "children"> & {
  child?: Component<U>;
  children?: import("svelte").Snippet;
};
export type WithChildren<T> = T & { children?: import("svelte").Snippet };
