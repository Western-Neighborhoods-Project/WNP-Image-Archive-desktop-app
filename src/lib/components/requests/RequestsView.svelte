<script lang="ts">
  import { onMount } from 'svelte';
  import { ordersResponse, ordersLoading, ordersError, refreshOrders } from '$lib/stores/requests';
  import { fulfillOrder, failOrder, type Order } from '$lib/commands/requests';
  import { formatCount } from '$lib/utils/format';

  // ── State ──────────────────────────────────────────────────────────────────
  let expandedOrderUuid = $state<string | null>(null);

  // Per-order action state: uuid → 'fulfilling' | 'failing' | null
  let actionState = $state<Record<string, 'fulfilling' | 'failing' | null>>({});
  let actionError = $state<Record<string, string | null>>({});

  // Fail reason dialog
  let failDialogUuid = $state<string | null>(null);
  let failReason = $state('');

  onMount(() => {
    refreshOrders();
  });

  function toggleExpand(uuid: string) {
    expandedOrderUuid = expandedOrderUuid === uuid ? null : uuid;
  }

  async function handleFulfill(uuid: string) {
    actionState = { ...actionState, [uuid]: 'fulfilling' };
    actionError = { ...actionError, [uuid]: null };
    try {
      await fulfillOrder(uuid);
      await refreshOrders();
    } catch (e) {
      actionError = { ...actionError, [uuid]: String(e) };
    } finally {
      actionState = { ...actionState, [uuid]: null };
    }
  }

  function openFailDialog(uuid: string) {
    failDialogUuid = uuid;
    failReason = '';
  }

  async function confirmFail() {
    if (!failDialogUuid) return;
    const uuid = failDialogUuid;
    failDialogUuid = null;

    actionState = { ...actionState, [uuid]: 'failing' };
    actionError = { ...actionError, [uuid]: null };
    try {
      await failOrder(uuid, failReason || 'Order marked as failed');
      await refreshOrders();
    } catch (e) {
      actionError = { ...actionError, [uuid]: String(e) };
    } finally {
      actionState = { ...actionState, [uuid]: null };
    }
  }

  function statusBadgeClass(status: string): string {
    switch (status) {
      case 'pending': return 'bg-yellow-100 text-yellow-800';
      case 'fulfilled': return 'bg-green-100 text-green-800';
      case 'failed': return 'bg-red-100 text-red-800';
      default: return 'bg-gray-100 text-gray-700';
    }
  }

  function resolutionLabel(res: string): string {
    switch (res) {
      case 'high': return 'High';
      case 'medium': return 'Medium';
      case 'low': return 'Low';
      default: return res;
    }
  }

  function formatDate(iso: string): string {
    try {
      return new Date(iso).toLocaleDateString('en-US', {
        year: 'numeric', month: 'short', day: 'numeric',
      });
    } catch {
      return iso;
    }
  }

  function formatCurrency(amount: number, currency: string): string {
    try {
      return new Intl.NumberFormat('en-US', { style: 'currency', currency }).format(amount);
    } catch {
      return `${currency} ${amount.toFixed(2)}`;
    }
  }

  const orders = $derived($ordersResponse?.data ?? []);
  const meta = $derived($ordersResponse?.meta ?? null);
</script>

<div class="flex h-full flex-col overflow-hidden">
  <!-- Header -->
  <div class="flex shrink-0 items-center justify-between border-b border-gray-200 bg-white px-6 py-4">
    <div>
      <h2 class="text-base font-semibold text-gray-900">Image Requests</h2>
      {#if meta}
        <p class="mt-0.5 text-xs text-gray-500">
          {formatCount(meta.total)} total
          {#if meta.fulfillable > 0}
            · <span class="font-medium text-blue-600">{meta.fulfillable} fulfillable</span>
          {/if}
        </p>
      {/if}
    </div>
    <button
      onclick={refreshOrders}
      disabled={$ordersLoading}
      class="flex items-center gap-1.5 rounded-md border border-gray-300 bg-white px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-50"
    >
      <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5 {$ordersLoading ? 'animate-spin' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
        <path stroke-linecap="round" stroke-linejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
      </svg>
      {$ordersLoading ? 'Refreshing…' : 'Refresh'}
    </button>
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-y-auto">
    {#if $ordersLoading && orders.length === 0}
      <div class="flex h-full items-center justify-center text-gray-400">
        <p>Loading orders…</p>
      </div>

    {:else if $ordersError}
      <div class="flex h-full flex-col items-center justify-center gap-3 text-center">
        <p class="text-sm text-red-600">{$ordersError}</p>
        <p class="text-xs text-gray-500">Check that the API URL is configured in Settings.</p>
        <button
          onclick={refreshOrders}
          class="rounded-md border border-gray-300 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50"
        >
          Try again
        </button>
      </div>

    {:else if orders.length === 0}
      <div class="flex h-full items-center justify-center text-gray-400">
        <p>No orders found.</p>
      </div>

    {:else}
      <div class="divide-y divide-gray-100">
        {#each orders as order (order.uuid)}
          {@const isExpanded = expandedOrderUuid === order.uuid}
          {@const busy = actionState[order.uuid] != null}
          <div class="bg-white">
            <!-- Order row -->
            <div class="flex items-start gap-4 px-6 py-4">
              <!-- Expand toggle -->
              <button
                onclick={() => toggleExpand(order.uuid)}
                class="mt-0.5 shrink-0 text-gray-400 hover:text-gray-600"
                title={isExpanded ? 'Collapse' : 'Expand'}
              >
                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 transition-transform {isExpanded ? 'rotate-90' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M9 5l7 7-7 7" />
                </svg>
              </button>

              <!-- Order info -->
              <div class="flex-1 min-w-0">
                <div class="flex flex-wrap items-center gap-2">
                  <span class="text-sm font-medium text-gray-900">{order.name}</span>
                  <span class="text-xs text-gray-400">{order.email}</span>
                  <span class="rounded-full px-2 py-0.5 text-xs font-medium {statusBadgeClass(order.status)}">
                    {order.status}
                  </span>
                </div>
                <div class="mt-0.5 flex flex-wrap items-center gap-3 text-xs text-gray-500">
                  <span>{order.item_count} image{order.item_count !== 1 ? 's' : ''}</span>
                  <span>{formatCurrency(order.total, order.currency)}</span>
                  <span>{formatDate(order.created_at)}</span>
                  <span class="font-mono text-gray-300">{order.uuid.slice(0, 8)}…</span>
                </div>

                {#if actionError[order.uuid]}
                  <p class="mt-1 text-xs text-red-600">{actionError[order.uuid]}</p>
                {/if}
              </div>

              <!-- Actions -->
              {#if order.status === 'processing'}
                <div class="flex shrink-0 items-center gap-2">
                  <button
                    onclick={() => handleFulfill(order.uuid)}
                    disabled={busy}
                    class="rounded-md bg-blue-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-700 disabled:opacity-50"
                  >
                    {actionState[order.uuid] === 'fulfilling' ? 'Fulfilling…' : 'Fulfill'}
                  </button>
                  <button
                    onclick={() => openFailDialog(order.uuid)}
                    disabled={busy}
                    class="rounded-md border border-gray-300 px-3 py-1.5 text-xs font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50"
                  >
                    Fail
                  </button>
                </div>
              {/if}
            </div>

            <!-- Expanded items -->
            {#if isExpanded}
              <div class="border-t border-gray-50 bg-gray-50/60 px-6 py-3">
                <table class="w-full text-xs">
                  <thead>
                    <tr class="text-left text-gray-400">
                      <th class="pb-1.5 pr-4 font-medium">Catalog #</th>
                      <th class="pb-1.5 pr-4 font-medium">Title</th>
                      <th class="pb-1.5 pr-4 font-medium">Resolution</th>
                      <th class="pb-1.5 font-medium text-right">Price</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-gray-100">
                    {#each order.items as item (item.catalog_number)}
                      <tr>
                        <td class="py-1.5 pr-4 font-mono text-gray-700">{item.catalog_number}</td>
                        <td class="py-1.5 pr-4 text-gray-600 max-w-[240px] truncate">{item.title ?? '—'}</td>
                        <td class="py-1.5 pr-4 text-gray-600">{resolutionLabel(item.resolution)}</td>
                        <td class="py-1.5 text-right text-gray-600">{formatCurrency(item.price, order.currency)}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<!-- Fail order dialog -->
{#if failDialogUuid}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
    <div class="w-full max-w-md rounded-xl bg-white p-6 shadow-xl">
      <h3 class="mb-2 text-sm font-semibold text-gray-900">Mark Order as Failed</h3>
      <p class="mb-4 text-xs text-gray-500">Optionally provide a reason for the failure.</p>
      <textarea
        bind:value={failReason}
        placeholder="Reason (optional)"
        rows={3}
        class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-800 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
      ></textarea>
      <div class="mt-4 flex justify-end gap-2">
        <button
          onclick={() => (failDialogUuid = null)}
          class="rounded-md border border-gray-300 px-4 py-2 text-sm text-gray-700 hover:bg-gray-50"
        >
          Cancel
        </button>
        <button
          onclick={confirmFail}
          class="rounded-md bg-red-600 px-4 py-2 text-sm text-white hover:bg-red-700"
        >
          Mark as Failed
        </button>
      </div>
    </div>
  </div>
{/if}
