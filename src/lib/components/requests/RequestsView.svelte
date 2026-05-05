<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    ordersResponse,
    ordersLoading,
    ordersError,
    refreshOrders,
  } from "$lib/stores/requests";
  import {
    fulfillOrder,
    failOrder,
    type Order,
  } from "$lib/commands/requests";
  import { isEditableTarget } from "$lib/utils/keyboardShortcuts";
  import { commandBarOpen } from "$lib/stores/commandBar";
  import { shortcutsHelpOpen } from "$lib/stores/shortcutsHelp";
  import { get } from "svelte/store";
  import { PageHeader } from "$lib/components/ui/page-header";
  import { Button } from "$lib/components/ui/button";
  import { Badge } from "$lib/components/ui/badge";
  import { Kbd } from "$lib/components/ui/kbd";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Check from "@lucide/svelte/icons/check";
  import X from "@lucide/svelte/icons/x";

  // ── State ──────────────────────────────────────────────────────────────────
  type StatusFilter = "processing" | "fulfilled" | "all";
  let statusFilter = $state<StatusFilter>("processing");
  let selectedUuid = $state<string | null>(null);

  // Per-order action state
  let actionState = $state<Record<string, "fulfilling" | "failing" | null>>({});
  let actionError = $state<Record<string, string | null>>({});

  // Fail reason dialog
  let failDialogUuid = $state<string | null>(null);
  let failReason = $state("");

  onMount(() => {
    refreshOrders();
  });

  // ⌘↵ — fulfill the currently selected processing order. View-scoped:
  // installed only while RequestsView is mounted, suppressed while
  // typing or while a dialog / command bar is open.
  function onWindowKeyDown(e: KeyboardEvent) {
    if (!(e.metaKey || e.ctrlKey)) return;
    if (e.key !== "Enter") return;
    if (failDialogUuid !== null) return; // mark-failed dialog handles its own keys
    if (get(commandBarOpen) || get(shortcutsHelpOpen)) return;
    if (isEditableTarget(e.target)) return;

    const order = selectedOrder;
    if (!order) return;
    if (order.status !== "processing") return;
    if (actionState[order.uuid] != null) return;

    e.preventDefault();
    handleFulfill(order.uuid);
  }

  onMount(() => {
    window.addEventListener("keydown", onWindowKeyDown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", onWindowKeyDown);
  });

  async function handleFulfill(uuid: string) {
    actionState = { ...actionState, [uuid]: "fulfilling" };
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
    failReason = "";
  }

  async function confirmFail() {
    if (!failDialogUuid) return;
    const uuid = failDialogUuid;
    failDialogUuid = null;
    actionState = { ...actionState, [uuid]: "failing" };
    actionError = { ...actionError, [uuid]: null };
    try {
      await failOrder(uuid, failReason || "Order marked as failed");
      await refreshOrders();
    } catch (e) {
      actionError = { ...actionError, [uuid]: String(e) };
    } finally {
      actionState = { ...actionState, [uuid]: null };
    }
  }

  function statusVariant(
    status: string,
  ): "warning" | "success" | "danger" | "secondary" | "info" | "default" {
    switch (status) {
      case "processing":
      case "pending":
        return "warning";
      case "fulfilled":
        return "success";
      case "failed":
        return "danger";
      default:
        return "secondary";
    }
  }

  function statusLabel(status: string): string {
    return status.charAt(0).toUpperCase() + status.slice(1);
  }

  function resolutionLabel(res: string): string {
    return { high: "High-res", medium: "Medium-res", low: "Low-res" }[res] ?? res;
  }

  function formatDate(iso: string): string {
    try {
      return new Date(iso).toLocaleDateString("en-US", {
        month: "short",
        day: "numeric",
        year: "numeric",
      });
    } catch {
      return iso;
    }
  }

  function formatRelative(iso: string): string {
    try {
      const then = new Date(iso);
      const seconds = Math.floor((Date.now() - then.getTime()) / 1000);
      if (seconds < 60) return `${seconds}s ago`;
      const minutes = Math.floor(seconds / 60);
      if (minutes < 60) return `${minutes}m ago`;
      const hours = Math.floor(minutes / 60);
      if (hours < 24) return `${hours}h ago`;
      const days = Math.floor(hours / 24);
      return `${days}d ago`;
    } catch {
      return "";
    }
  }

  function formatCurrency(amount: number, currency: string): string {
    try {
      return new Intl.NumberFormat("en-US", {
        style: "currency",
        currency,
      }).format(amount);
    } catch {
      return `${currency} ${amount.toFixed(2)}`;
    }
  }

  // ── Derived data ───────────────────────────────────────────────────────────
  const allOrders = $derived($ordersResponse?.data ?? []);
  const meta = $derived($ordersResponse?.meta ?? null);

  const filteredOrders = $derived.by(() => {
    if (statusFilter === "all") return allOrders;
    return allOrders.filter((o) => o.status === statusFilter);
  });

  $effect(() => {
    // Auto-select the first filtered order if none selected, or selected isn't in list
    if (filteredOrders.length === 0) {
      selectedUuid = null;
      return;
    }
    if (
      selectedUuid === null ||
      !filteredOrders.some((o) => o.uuid === selectedUuid)
    ) {
      selectedUuid = filteredOrders[0].uuid;
    }
  });

  const selectedOrder = $derived.by(() => {
    return filteredOrders.find((o) => o.uuid === selectedUuid) ?? null;
  });

  const counts = $derived({
    processing: allOrders.filter((o) => o.status === "processing").length,
    fulfilled: allOrders.filter((o) => o.status === "fulfilled").length,
    all: allOrders.length,
  });
</script>

<div class="flex flex-1 flex-col min-w-0 min-h-0">
  <PageHeader
    title="Pending requests"
    count={meta
      ? `${meta.fulfillable} unreviewed · ${counts.fulfilled} fulfilled`
      : "Loading…"}
  >
    {#snippet right()}
      <Button
        size="xs"
        variant="outline"
        disabled={$ordersLoading}
        onclick={refreshOrders}
      >
        <RefreshCw class={$ordersLoading ? "animate-spin" : ""} />
        {$ordersLoading ? "Refreshing…" : "Check now"}
      </Button>
    {/snippet}
  </PageHeader>

  <!-- Filter tabs -->
  <div
    class="flex items-end px-5 gap-5 border-b border-border bg-background flex-shrink-0"
  >
    {#each [{ key: "processing" as StatusFilter, label: "Processing", count: counts.processing }, { key: "fulfilled" as StatusFilter, label: "Fulfilled", count: counts.fulfilled }, { key: "all" as StatusFilter, label: "All", count: counts.all }] as tab (tab.key)}
      <button
        type="button"
        onclick={() => (statusFilter = tab.key)}
        class="py-3.5 flex items-center gap-1.5 text-[13px] font-medium border-b-2 -mb-px transition-colors
          {statusFilter === tab.key
          ? 'text-foreground border-foreground'
          : 'text-muted-foreground border-transparent hover:text-foreground'}"
      >
        {tab.label}
        {#if tab.count > 0}
          <Badge variant="secondary">{tab.count}</Badge>
        {/if}
      </button>
    {/each}
  </div>

  {#if $ordersError}
    <div class="flex flex-1 items-center justify-center p-10">
      <div class="text-center">
        <p class="text-destructive text-sm mb-2">{$ordersError}</p>
        <p class="text-muted-foreground text-xs mb-4">
          Check that the API URL is configured in Settings.
        </p>
        <Button variant="outline" onclick={refreshOrders}>Try again</Button>
      </div>
    </div>
  {:else if filteredOrders.length === 0}
    <div class="flex-1 flex items-center justify-center text-muted-foreground">
      <p class="text-sm">
        No {statusFilter === "all" ? "" : statusFilter} orders.
      </p>
    </div>
  {:else}
    <!-- Two-pane layout: orders list + detail -->
    <div class="flex flex-1 min-h-0">
      <!-- Orders list -->
      <div
        class="w-[360px] flex-shrink-0 border-r border-border bg-sidebar-bg flex flex-col"
      >
        <div class="flex-1 overflow-auto">
          {#each filteredOrders as order (order.uuid)}
            {@const isSelected = order.uuid === selectedUuid}
            <button
              type="button"
              onclick={() => (selectedUuid = order.uuid)}
              class="w-full text-left px-3.5 pt-3.5 pb-3 border-b border-border-muted transition-colors
                {isSelected
                ? 'bg-background border-l-2 border-l-foreground'
                : 'border-l-2 border-l-transparent hover:bg-hover'}"
            >
              <!-- Header row -->
              <div class="flex items-baseline gap-1.5 mb-1.5">
                <div
                  class="font-mono text-[10.5px] text-muted-foreground font-medium"
                >
                  #{order.order_number}
                </div>
                <div class="flex-1"></div>
                <div class="text-[10.5px] text-muted-foreground">
                  {formatRelative(order.created_at)}
                </div>
              </div>
              <!-- Requester -->
              <div
                class="text-[13px] font-medium text-foreground truncate"
              >
                {order.name}
              </div>
              <div
                class="text-[11.5px] text-muted-foreground truncate mb-2"
              >
                {order.email}
              </div>
              <!-- Status + count -->
              <div class="flex items-center gap-2">
                <Badge variant={statusVariant(order.status)}>
                  {statusLabel(order.status)}
                </Badge>
                <div class="flex-1"></div>
                <div
                  class="text-[11px] text-muted-foreground tabular-nums"
                >
                  {order.item_count}
                  {order.item_count === 1 ? "image" : "images"}
                </div>
              </div>
            </button>
          {/each}
        </div>
      </div>

      <!-- Detail pane -->
      <div class="flex-1 flex flex-col bg-background min-w-0 select-text">
        {#if selectedOrder}
          <!-- Order header -->
          <div class="px-6 pt-5 pb-4 border-b border-border">
            <div class="flex items-start gap-4">
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2.5 mb-1">
                  <Badge variant={statusVariant(selectedOrder.status)}>
                    {statusLabel(selectedOrder.status)}
                  </Badge>
                  <span class="font-mono text-xs text-muted-foreground">
                    Order #{selectedOrder.order_number}
                  </span>
                  <span class="text-xs text-muted-foreground">
                    · {selectedOrder.item_count}
                    {selectedOrder.item_count === 1 ? "image" : "images"}
                  </span>
                </div>
                <div
                  class="text-[19px] font-semibold text-foreground tracking-[-0.3px] mb-0.5"
                >
                  {selectedOrder.name}
                </div>
                <div class="text-[13px] text-muted-foreground">
                  <span class="font-mono">{selectedOrder.email}</span>
                  · {formatDate(selectedOrder.created_at)}
                </div>
              </div>
              {#if selectedOrder.status === "processing"}
                <div class="flex gap-2 flex-shrink-0">
                  <Button
                    size="xs"
                    variant="outline"
                    disabled={actionState[selectedOrder.uuid] != null}
                    onclick={() => openFailDialog(selectedOrder!.uuid)}
                  >
                    Mark failed
                  </Button>
                  <Button
                    size="xs"
                    disabled={actionState[selectedOrder.uuid] != null}
                    onclick={() => handleFulfill(selectedOrder!.uuid)}
                  >
                    <Check />
                    {actionState[selectedOrder.uuid] === "fulfilling"
                      ? "Fulfilling…"
                      : "Fulfill order"}
                    <Kbd dim>⌘↵</Kbd>
                  </Button>
                </div>
              {/if}
            </div>
            {#if actionError[selectedOrder.uuid]}
              <p class="text-destructive text-xs mt-3">
                {actionError[selectedOrder.uuid]}
              </p>
            {/if}
          </div>

          <!-- Body: order details -->
          <div class="flex-1 overflow-auto">
            <div class="px-6 py-5 border-b border-border-muted">
              <div
                class="text-[11px] font-semibold uppercase tracking-[0.4px] text-muted-foreground mb-3"
              >
                Order details
              </div>
              <div class="grid grid-cols-3 gap-x-6 gap-y-3.5">
                <div>
                  <div class="text-[11px] text-muted-foreground mb-0.5">
                    Total
                  </div>
                  <div class="text-[13px] text-foreground tabular-nums">
                    {formatCurrency(
                      selectedOrder.total,
                      selectedOrder.currency,
                    )}
                  </div>
                </div>
                <div>
                  <div class="text-[11px] text-muted-foreground mb-0.5">
                    Submitted
                  </div>
                  <div class="text-[13px] text-foreground">
                    {formatDate(selectedOrder.created_at)}
                  </div>
                </div>
                <div>
                  <div class="text-[11px] text-muted-foreground mb-0.5">
                    Paid
                  </div>
                  <div class="text-[13px] text-foreground">
                    {selectedOrder.paid_at
                      ? formatDate(selectedOrder.paid_at)
                      : "—"}
                  </div>
                </div>
              </div>
            </div>

            <!-- Image grid -->
            <div class="px-6 py-5">
              <div
                class="text-[11px] font-semibold uppercase tracking-[0.4px] text-muted-foreground mb-3"
              >
                Requested images
              </div>
              <div class="grid grid-cols-3 gap-3">
                {#each selectedOrder.items as item, i (i)}
                  <div
                    class="rounded-lg overflow-hidden border border-border bg-background"
                  >
                    <div
                      class="aspect-[4/3] bg-secondary flex items-center justify-center font-mono text-xs text-muted-foreground relative"
                    >
                      <div
                        class="pointer-events-none absolute bottom-1.5 left-1.5 font-mono text-[10px] font-medium"
                        style="color: rgba(0,0,0,0.65);"
                      >
                        {item.catalog_number}
                      </div>
                    </div>
                    <div class="p-2.5">
                      <div class="text-[12.5px] font-medium truncate">
                        {item.title ?? "(untitled)"}
                      </div>
                      <div
                        class="text-[11px] text-muted-foreground flex items-center gap-1.5 justify-between mt-0.5"
                      >
                        <span>{resolutionLabel(item.resolution)}</span>
                        <span class="tabular-nums">
                          {formatCurrency(item.price, selectedOrder.currency)}
                        </span>
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<!-- Mark-failed dialog -->
{#if failDialogUuid}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
    <div
      class="w-full max-w-md rounded-lg bg-background p-6 border border-border"
      style="box-shadow: 0 24px 64px rgba(0,0,0,0.3);"
    >
      <h3 class="mb-2 text-base font-semibold text-foreground">
        Mark order as failed
      </h3>
      <p class="mb-4 text-xs text-muted-foreground">
        Optionally provide a reason. The requester will be notified.
      </p>
      <textarea
        bind:value={failReason}
        placeholder="Reason (optional)"
        rows={3}
        class="w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
      ></textarea>
      <div class="mt-4 flex justify-end gap-2">
        <Button variant="outline" onclick={() => (failDialogUuid = null)}>
          <Kbd dim>Esc</Kbd>
          Cancel
        </Button>
        <Button variant="destructive" onclick={confirmFail}>
          <X class="size-3.5" />
          Mark as failed
        </Button>
      </div>
    </div>
  </div>
{/if}
