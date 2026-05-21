<script lang="ts">
  const repositoryUrl = "https://github.com/Chaotic-Bastard-Creations/whispra";

  const links = [
    { label: "Documentation", href: `${repositoryUrl}#running-the-bridge` },
    { label: "Self-Host", href: `${repositoryUrl}#running-the-bridge` },
    { label: "Roadmap", href: `${repositoryUrl}#features` }
  ];
</script>

<section class="workspace-view" aria-label="About Whispra">
  <div class="settings-view">
    <nav class="settings-nav" aria-label="Settings">
      <a class="settings-nav-item" href="/settings/security">Security</a>
      <a class="settings-nav-item is-active" href="/settings/about">About Whispra</a>
    </nav>

    <article class="security-panel">
      <header class="security-header">
        <h1>About Whispra</h1>
      </header>

      <section class="security-section">
        <h2>Project links</h2>
        <ul>
          {#each links as link}
            <li>
              <a href={link.href} target="_blank" rel="noreferrer">{link.label}</a>
            </li>
          {/each}
        </ul>
      </section>
    </article>
  </div>
</section>
