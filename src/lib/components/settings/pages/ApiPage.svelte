<script lang="ts">
  import { onMount } from "svelte";
  import { getSetting, setSetting } from "$lib/commands/settings";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";

  let laravelApiUrl = $state("");
  let laravelApiToken = $state("");
  let s3Endpoint = $state("");
  let s3Bucket = $state("");
  let s3AccessKey = $state("");
  let s3SecretKey = $state("");
  let s3Region = $state("");
  let s3PublicBaseUrl = $state("");

  let saveStatus = $state<"idle" | "saving" | "saved">("idle");
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let saveError = $state<string | null>(null);

  onMount(async () => {
    const [apiUrl, apiToken, endpoint, bucket, accessKey, secretKey, region, publicUrl] =
      await Promise.all([
        getSetting("laravel_api_url"),
        getSetting("laravel_api_token"),
        getSetting("s3_endpoint"),
        getSetting("s3_bucket"),
        getSetting("s3_access_key"),
        getSetting("s3_secret_key"),
        getSetting("s3_region"),
        getSetting("s3_public_base_url"),
      ]);
    laravelApiUrl = apiUrl ?? "";
    laravelApiToken = apiToken ?? "";
    s3Endpoint = endpoint ?? "";
    s3Bucket = bucket ?? "";
    s3AccessKey = accessKey ?? "";
    s3SecretKey = secretKey ?? "";
    s3Region = region ?? "";
    s3PublicBaseUrl = publicUrl ?? "";
  });

  async function save() {
    saveStatus = "saving";
    saveError = null;
    clearTimeout(saveTimer);

    // Save serially so we can pinpoint which key failed. Secret keys
    // (api token, s3 keys) hit the macOS keychain via the backend; if the
    // OS denies access in dev builds we want to surface that explicitly,
    // not swallow it.
    const fields: Array<[string, string]> = [
      ["laravel_api_url", laravelApiUrl],
      ["laravel_api_token", laravelApiToken],
      ["s3_endpoint", s3Endpoint],
      ["s3_bucket", s3Bucket],
      ["s3_access_key", s3AccessKey],
      ["s3_secret_key", s3SecretKey],
      ["s3_region", s3Region],
      ["s3_public_base_url", s3PublicBaseUrl],
    ];

    for (const [key, value] of fields) {
      try {
        await setSetting(key, value);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        saveError = `Failed to save '${key}': ${msg}`;
        console.error("ApiPage save failed", key, e);
        saveStatus = "idle";
        return;
      }
    }

    saveStatus = "saved";
    saveTimer = setTimeout(() => (saveStatus = "idle"), 2000);
  }
</script>

<div class="max-w-[640px] space-y-6">
  <section>
    <h3 class="text-[14px] font-semibold text-foreground mb-1">
      OpenSFHistory API
    </h3>
    <p class="text-[12px] text-muted-foreground mb-3">
      Base URL and auth token for the public archive's Laravel API. Image-use
      orders come from this endpoint; the token is sent as
      <code class="font-mono text-[11px] px-1 py-0.5 rounded bg-secondary"
        >Authorization: Bearer …</code
      > on every request.
    </p>
    <div class="space-y-3.5">
      <div class="space-y-1.5">
        <label
          for="api-url"
          class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
          >Base URL</label
        >
        <Input
          id="api-url"
          bind:value={laravelApiUrl}
          placeholder="https://opensfhistory.org"
        />
      </div>
      <div class="space-y-1.5">
        <label
          for="api-token"
          class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
          >API token</label
        >
        <Input
          id="api-token"
          type="password"
          bind:value={laravelApiToken}
          placeholder="Personal access token from OpenSFHistory"
          autocomplete="off"
        />
        <p class="text-[11px] text-muted-foreground">
          Get this from your OpenSFHistory user settings. Required for fetching
          orders, fulfilling, and posting back results.
        </p>
      </div>
    </div>
  </section>

  <section class="border-t border-border pt-5">
    <h3 class="text-[14px] font-semibold text-foreground mb-1">
      S3-compatible storage (Backblaze B2)
    </h3>
    <p class="text-[12px] text-muted-foreground mb-3">
      Order fulfillments and ad-hoc shares upload here. Backblaze B2 is the
      planned provider.
    </p>
    <div class="space-y-3.5">
      <div class="space-y-1.5">
        <label
          for="s3-endpoint"
          class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
          >Endpoint URL</label
        >
        <Input
          id="s3-endpoint"
          bind:value={s3Endpoint}
          placeholder="https://s3.us-west-001.backblazeb2.com"
        />
      </div>
      <div class="grid grid-cols-2 gap-3">
        <div class="space-y-1.5">
          <label
            for="s3-bucket"
            class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
            >Bucket</label
          >
          <Input id="s3-bucket" bind:value={s3Bucket} placeholder="wnp-archive" />
        </div>
        <div class="space-y-1.5">
          <label
            for="s3-region"
            class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
            >Region</label
          >
          <Input id="s3-region" bind:value={s3Region} placeholder="us-west-001" />
        </div>
      </div>
      <div class="space-y-1.5">
        <label
          for="s3-access-key"
          class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
          >Access key ID</label
        >
        <Input id="s3-access-key" bind:value={s3AccessKey} />
      </div>
      <div class="space-y-1.5">
        <label
          for="s3-secret-key"
          class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
          >Secret access key</label
        >
        <Input
          id="s3-secret-key"
          type="password"
          bind:value={s3SecretKey}
        />
      </div>
      <div class="space-y-1.5">
        <label
          for="s3-public-url"
          class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
          >Public base URL</label
        >
        <Input
          id="s3-public-url"
          bind:value={s3PublicBaseUrl}
          placeholder="https://files.opensfhistory.org"
        />
        <p class="text-[11px] text-muted-foreground">
          URL prefix used to build download links (e.g. CDN hostname).
        </p>
      </div>
    </div>
  </section>

  <div class="space-y-2 pt-2">
    <div class="flex items-center gap-3">
      <Button disabled={saveStatus === "saving"} onclick={save}>
        {saveStatus === "saving" ? "Saving…" : "Save"}
      </Button>
      {#if saveStatus === "saved"}
        <span class="text-sm text-success">Saved</span>
      {/if}
    </div>
    {#if saveError}
      <p class="text-[12px] text-destructive">{saveError}</p>
    {/if}
  </div>
</div>
