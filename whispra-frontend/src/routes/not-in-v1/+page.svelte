<script lang="ts">
  import { page } from "$app/state";

  const feature = $derived(page.url.searchParams.get("feature") || "This feature");
</script>

<section class="workspace-view" aria-label="Not in v1">
  <div class="panel-stub">
    <p>{feature} is not in v1.</p>
  </div>
</section>
