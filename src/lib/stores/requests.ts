import { writable } from 'svelte/store';
import { fetchOrders, type OrdersResponse } from '$lib/commands/requests';

export const ordersResponse = writable<OrdersResponse | null>(null);
export const ordersLoading = writable(false);
export const ordersError = writable<string | null>(null);

export async function refreshOrders(): Promise<void> {
  ordersLoading.set(true);
  ordersError.set(null);
  try {
    const result = await fetchOrders();
    ordersResponse.set(result);
  } catch (e) {
    ordersError.set(String(e));
  } finally {
    ordersLoading.set(false);
  }
}
