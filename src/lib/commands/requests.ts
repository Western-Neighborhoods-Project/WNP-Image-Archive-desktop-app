import { invoke } from '@tauri-apps/api/core';

export interface OrderItem {
  catalog_number: string;
  title: string | null;
  resolution: string; // "high" | "medium" | "low"
  price_cents: number;
  price: number;
}

export interface Order {
  uuid: string;
  name: string;
  email: string;
  status: string; // "pending" | "fulfilled" | "failed"
  total_cents: number;
  total: number;
  currency: string;
  item_count: number;
  created_at: string;
  paid_at: string | null;
  items: OrderItem[];
}

export interface OrdersMeta {
  total: number;
  fulfillable: number;
}

export interface OrdersResponse {
  data: Order[];
  meta: OrdersMeta;
}

export interface FulfillResult {
  uuid: string;
  zip_url: string;
  items_fulfilled: number;
}

export async function fetchOrders(): Promise<OrdersResponse> {
  return invoke('fetch_orders');
}

export async function fulfillOrder(uuid: string): Promise<FulfillResult> {
  return invoke('fulfill_order', { uuid });
}

export async function failOrder(uuid: string, reason: string): Promise<void> {
  return invoke('fail_order', { uuid, reason });
}
