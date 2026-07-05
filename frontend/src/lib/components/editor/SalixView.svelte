<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import * as d3Hierarchy from 'd3-hierarchy';
  import * as d3Force from 'd3-force';
  import * as ipc from '../../services/ipc';
  import { listen } from '@tauri-apps/api/event';
  import { salixActive, notesList, loadNotes } from '../../stores/notes';

  // State
  let canvas = $state<HTMLCanvasElement | null>(null);
  let searchInput = $state<HTMLInputElement | null>(null);
  let searchQuery = $state('');
  let showSearch = $state(false);
  let sproutDialog = $state<{ parentPath: string; x: number; y: number } | null>(null);
  let sproutName = $state('');
  let sproutType = $state<'note' | 'folder'>('note');
  let activeSplitNote = $state<any | null>(null);
  let width = $state(800);
  let height = $state(600);

  // Simulation variables
  let simulation: any = null;
  let nodes: any[] = [];
  let links: any[] = [];
  let d3Root: any = null;
  let collapsedPaths = $state<Set<string>>(new Set());

  // Physics Control Variables (Obsidian-Style)
  let forceGravity = $state(-150);
  let forceLinkDistance = $state(50);
  let forceCollisionRadius = $state(12);
  let showPhysicsPanel = $state(false);

  // Mouse interaction
  let transform = $state({ x: 0, y: 0, k: 1 });
  let draggedNode = $state<any | null>(null);
  let dragStartMouse = { x: 0, y: 0 };
  let dragStartNodePos = { x: 0, y: 0 };
  let hoveredNode = $state<any | null>(null);
  let rightClickedNode = $state<any | null>(null);
  
  // Snap Target for Drag & Drop
  let snappedTarget = $state<any | null>(null);

  // Particles for AI telemetry
  let particles: any[] = [];

  // Listen to Tauri resize and layout transitions
  let observer: ResizeObserver | null = null;
  let isZenTransitioning = $state(false);

  // Load Graph Data
  async function reloadGraph() {
    try {
      const data = await ipc.getGraphNodes();
      // Compile D3 radial layout
      const hRoot = d3Hierarchy.hierarchy(data, (d: any) => {
        if (collapsedPaths.has(d.file_path)) return null;
        return d.children;
      });

      // Calculate radial angles and radii
      const sizeRadius = Math.min(width, height) / 2 - 80;
      const cluster = d3Hierarchy.cluster().size([2 * Math.PI, Math.max(100, sizeRadius)]);
      cluster(hRoot);

      d3Root = hRoot;

      // Map to physics nodes
      const oldNodesMap = new Map(nodes.map(n => [n.data.id || n.data.file_path, n]));
      
      const newNodes: any[] = [];
      hRoot.descendants().forEach((d: any) => {
        const id = d.data.id || d.data.file_path;
        const oldNode: any = oldNodesMap.get(id);

        const angle = d.x;
        const radius = d.y;
        
        // Root is centered, children projected outward
        const targetX = d.depth === 0 ? 0 : radius * Math.cos(angle - Math.PI / 2);
        const targetY = d.depth === 0 ? 0 : radius * Math.sin(angle - Math.PI / 2);

        newNodes.push({
          id,
          x: oldNode ? oldNode.x : targetX,
          y: oldNode ? oldNode.y : targetY,
          vx: oldNode ? oldNode.vx : 0,
          vy: oldNode ? oldNode.vy : 0,
          targetX,
          targetY,
          data: d.data,
          depth: d.depth,
          is_encrypted: d.data.is_encrypted,
          char_size: d.data.char_size || 0
        });
      });

      const newLinks: any[] = [];
      hRoot.links().forEach((link: any) => {
        const sourceId = link.source.data.id || link.source.data.file_path;
        const targetId = link.target.data.id || link.target.data.file_path;
        newLinks.push({
          source: newNodes.find(n => n.id === sourceId),
          target: newNodes.find(n => n.id === targetId)
        });
      });

      nodes = newNodes;
      links = newLinks;

      if (simulation) {
        simulation.nodes(nodes);
        simulation.force("link").links(links);
        simulation.alpha(0.3).restart();
      }
    } catch (e) {
      console.error("Failed to load graph nodes:", e);
    }
  }

  // Setup simulation
  function initSimulation() {
    simulation = d3Force.forceSimulation(nodes)
      .force("link", d3Force.forceLink(links).distance(forceLinkDistance).strength(0.8))
      .force("charge", d3Force.forceManyBody().strength(forceGravity))
      .force("collide", d3Force.forceCollide().radius((d: any) => {
        // Incorporating the 35% scaling reduction into collision parameters
        const baseRadius = d.data.is_folder ? 22 : 12;
        return baseRadius * 0.65;
      }))
      .force("x", d3Force.forceX((d: any) => d.targetX).strength(0.4))
      .force("y", d3Force.forceY((d: any) => d.targetY).strength(0.4))
      .on("tick", draw);
  }

  // Reactively bind physics control variables to forces
  $effect(() => {
    if (simulation) {
      simulation.force("charge", d3Force.forceManyBody().strength(forceGravity));
      simulation.force("link").distance(forceLinkDistance);
      simulation.force("collide", d3Force.forceCollide().radius((d: any) => {
        return d.data.is_folder ? forceCollisionRadius * 1.83 * 0.65 : forceCollisionRadius * 0.65;
      }));
      simulation.alpha(0.1).restart();
    }
  });

  // Draw loop
  function draw() {
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // Dynamic CSS Variable Extraction
    const style = window.getComputedStyle(canvas);
    const themeAccent = style.getPropertyValue("--accent") || "#0a84ff";
    const themeText = style.getPropertyValue("--text-primary") || "#ffffff";
    const themeTextSecondary = style.getPropertyValue("--text-secondary") || "rgba(255, 255, 255, 0.75)";

    ctx.save();
    ctx.clearRect(0, 0, width, height);
    
    // Grid background
    drawGrid(ctx);

    ctx.translate(width / 2 + transform.x, height / 2 + transform.y);
    ctx.scale(transform.k, transform.k);

    // Filter logic
    const hasQuery = searchQuery.trim().length > 0;
    const lowerQuery = searchQuery.toLowerCase();

    // Draw Links
    links.forEach(l => {
      let opacity = 0.4;
      if (hasQuery) {
        const sourceMatch = l.source.data.name.toLowerCase().includes(lowerQuery);
        const targetMatch = l.target.data.name.toLowerCase().includes(lowerQuery);
        opacity = (sourceMatch || targetMatch) ? 0.9 : 0.15;
      }
      ctx.beginPath();
      ctx.strokeStyle = themeAccent;
      ctx.globalAlpha = opacity;
      ctx.lineWidth = 1.2;
      ctx.moveTo(l.source.x, l.source.y);
      ctx.lineTo(l.target.x, l.target.y);
      ctx.stroke();
      ctx.globalAlpha = 1.0;
    });

    // Draw Nodes
    nodes.forEach(n => {
      const isFolder = n.data.is_folder;
      const isEncrypted = n.is_encrypted;
      
      let opacity = 1.0;
      let glow = false;
      if (hasQuery) {
        const match = n.data.name.toLowerCase().includes(lowerQuery);
        opacity = match ? 1.0 : 0.15;
        glow = match;
      }

      ctx.save();
      ctx.globalAlpha = opacity;

      // Glow effect if matched
      if (glow) {
        ctx.shadowBlur = 15;
        ctx.shadowColor = "#30d158";
      }

      // Draw node shape (downscaled by 35%)
      ctx.beginPath();
      const radius = isFolder ? (14 * 0.65) : (7 * 0.65);
      ctx.arc(n.x, n.y, radius, 0, 2 * Math.PI);

      if (isFolder) {
        ctx.fillStyle = collapsedPaths.has(n.data.file_path) ? "#ff9f0a" : themeAccent;
      } else if (isEncrypted) {
        ctx.fillStyle = "#ff453a"; // Locked fruits (crimson-neon)
      } else {
        ctx.fillStyle = "#30d158"; // Note leaves (green)
      }

      ctx.fill();
      ctx.strokeStyle = "#ffffff";
      ctx.lineWidth = 1.0;
      ctx.stroke();

      // Render Lock Glyph overlay for encrypted notes
      if (isEncrypted) {
        ctx.fillStyle = "#ffffff";
        ctx.font = "bold 6px system-ui";
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.fillText("🔒", n.x, n.y);
      }

      // Node label - Zoom Dependent LOD Text Blackout Clause (k < 0.6)
      if (transform.k >= 0.6) {
        ctx.shadowBlur = 0;
        ctx.fillStyle = hoveredNode === n ? themeText : themeTextSecondary;
        ctx.font = "10px var(--font-sans, system-ui)";
        ctx.textAlign = "center";
        ctx.fillText(n.data.name, n.x, n.y + radius + 11);
      }

      ctx.restore();
    });

    // Snapping logic visual overlay during drag
    if (draggedNode && snappedTarget) {
      ctx.save();
      ctx.beginPath();
      ctx.arc(snappedTarget.x, snappedTarget.y, 14 * 0.65 + 6, 0, 2 * Math.PI);
      ctx.strokeStyle = "#00ffff";
      ctx.lineWidth = 2.0;
      ctx.shadowBlur = 12;
      ctx.shadowColor = "#00ffff";
      ctx.stroke();
      ctx.restore();
    } else if (draggedNode && hoveredNode && hoveredNode !== draggedNode && hoveredNode.data.is_folder) {
      // Free drag connection indicator (non-snapped)
      ctx.beginPath();
      ctx.strokeStyle = "rgba(255, 255, 255, 0.3)";
      ctx.lineWidth = 1.0;
      ctx.setLineDash([3, 3]);
      ctx.moveTo(draggedNode.x, draggedNode.y);
      ctx.lineTo(hoveredNode.x, hoveredNode.y);
      ctx.stroke();
      ctx.setLineDash([]);
    }

    // Draw AI Telemetry Trace Particles
    updateAndDrawParticles(ctx);

    ctx.restore();
  }

  function drawGrid(ctx: CanvasRenderingContext2D) {
    ctx.strokeStyle = "rgba(255, 255, 255, 0.02)";
    ctx.lineWidth = 1;
    const size = 40;
    
    const startX = -transform.x - (width / 2);
    const startY = -transform.y - (height / 2);
    
    for (let x = startX - (startX % size); x < width + size; x += size) {
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, height);
      ctx.stroke();
    }
    for (let y = startY - (startY % size); y < height + size; y += size) {
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();
    }
  }

  function updateAndDrawParticles(ctx: CanvasRenderingContext2D) {
    const activeParticles: any[] = [];
    particles.forEach(p => {
      p.progress += 0.02; // Slightly higher velocity
      if (p.progress < 1.0) {
        const pathIndex = Math.min(
          p.path.length - 1,
          Math.floor(p.progress * p.path.length)
        );
        const segmentProgress = (p.progress * p.path.length) % 1.0;
        
        const nodeA = pathIndex === 0 ? p.startNode : p.path[pathIndex - 1];
        const nodeB = p.path[pathIndex];

        if (nodeA && nodeB) {
          const x = nodeA.x + (nodeB.x - nodeA.x) * segmentProgress;
          const y = nodeA.y + (nodeB.y - nodeA.y) * segmentProgress;
          
          // High-intensity neon particle glow arrays
          ctx.save();
          ctx.beginPath();
          ctx.arc(x, y, 4.5, 0, 2 * Math.PI);
          ctx.fillStyle = "#00ffff";
          ctx.shadowBlur = 12;
          ctx.shadowColor = "#00ffff";
          ctx.fill();
          ctx.restore();
        }
        activeParticles.push(p);
      }
    });
    particles = activeParticles;
  }

  // Triggers telemetry trace particle flowing based on action routing directions
  function triggerTraceParticle(noteId: string, action: string) {
    const startNode = nodes.find(n => n.id === noteId);
    if (!startNode) return;

    // Find hierarchical path to root
    const path: any[] = [];
    let current = startNode;
    
    let nextParent = links.find(l => l.target === current)?.source;
    while (nextParent) {
      path.push(nextParent);
      current = nextParent;
      nextParent = links.find(l => l.target === current)?.source;
    }

    if (path.length > 0) {
      // READ transactions travel inward (leaf -> root)
      // WRITE/EDIT transactions travel outward (root -> leaf)
      if (action === "read") {
        particles.push({
          startNode,
          path,
          progress: 0
        });
      } else {
        // Reverse trace path: start at root, travel to startNode
        const reversedPath = [...path].reverse();
        const first = reversedPath[0];
        const rest = [...reversedPath.slice(1), startNode];
        particles.push({
          startNode: first,
          path: rest,
          progress: 0
        });
      }
    }
  }

  // Handle Resize and Zen-Mode Animation Guard
  function handleResize() {
    if (!canvas) return;
    width = canvas.parentElement?.clientWidth || 800;
    height = canvas.parentElement?.clientHeight || 600;
    canvas.width = width;
    canvas.height = height;
    draw();
  }

  // Mouse coordinate mappings
  function getMouseCoords(e: MouseEvent) {
    if (!canvas) return { x: 0, y: 0 };
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left - width / 2;
    const y = e.clientY - rect.top - height / 2;
    return {
      x: (x - transform.x) / transform.k,
      y: (y - transform.y) / transform.k
    };
  }

  // Absolute Event Segregation
  let isDraggingNode = false;

  function onMouseDown(e: MouseEvent) {
    if (e.button === 2) return; // Context menu
    const coords = getMouseCoords(e);
    
    // Check if clicked node (incorporating 35% scaling reduction)
    const clicked = nodes.find(n => {
      const dist = Math.hypot(n.x - coords.x, n.y - coords.y);
      return dist <= (n.data.is_folder ? (14 * 0.65) : (7 * 0.65));
    });

    if (clicked) {
      draggedNode = clicked;
      isDraggingNode = false; // Reset drag state guard
      dragStartMouse = { x: e.clientX, y: e.clientY };
      dragStartNodePos = { x: clicked.x, y: clicked.y };
      if (simulation) simulation.alphaTarget(0.3).restart();
    } else {
      dragStartMouse = { x: e.clientX, y: e.clientY };
    }
  }

  function onMouseMove(e: MouseEvent) {
    const coords = getMouseCoords(e);

    // Hover check (incorporating 35% scaling reduction)
    hoveredNode = nodes.find(n => {
      const dist = Math.hypot(n.x - coords.x, n.y - coords.y);
      return dist <= (n.data.is_folder ? (14 * 0.65) : (7 * 0.65));
    }) || null;

    if (draggedNode) {
      isDraggingNode = true; // Block click triggers
      
      const dx = (e.clientX - dragStartMouse.x) / transform.k;
      const dy = (e.clientY - dragStartMouse.y) / transform.k;
      const proposedX = dragStartNodePos.x + dx;
      const proposedY = dragStartNodePos.y + dy;

      // Magnetic snapping anchor mechanism (Distance < 25px)
      snappedTarget = null;
      for (const n of nodes) {
        if (n !== draggedNode && n.data.is_folder) {
          const dist = Math.hypot(n.x - proposedX, n.y - proposedY);
          if (dist < 25) {
            snappedTarget = n;
            break;
          }
        }
      }

      if (snappedTarget) {
        draggedNode.x = snappedTarget.x;
        draggedNode.y = snappedTarget.y;
      } else {
        draggedNode.x = proposedX;
        draggedNode.y = proposedY;
      }

      if (simulation) simulation.alpha(0.3).restart();
    } else if (e.buttons === 1) {
      // Pan canvas
      transform.x += e.clientX - dragStartMouse.x;
      transform.y += e.clientY - dragStartMouse.y;
      dragStartMouse = { x: e.clientX, y: e.clientY };
      draw();
    }
    draw();
  }

  async function onMouseUp(e: MouseEvent) {
    if (draggedNode) {
      // Only trigger grafting if snapped target is active on termination
      if (snappedTarget && snappedTarget !== draggedNode) {
        const oldPath = draggedNode.data.file_path;
        const parentFolder = snappedTarget.data.file_path;
        
        let newPath = "";
        const parts = oldPath.split('/');
        const fileName = parts[parts.length - 1];

        if (parentFolder === "") {
          newPath = fileName;
        } else {
          newPath = `${parentFolder}/${fileName}`;
        }

        if (confirm(`Move "${draggedNode.data.name}" to "${snappedTarget.data.name}"?`)) {
          try {
            await ipc.graftNode(oldPath, newPath);
            await reloadGraph();
            await loadNotes();
          } catch (err: any) {
            alert(`Grafting failed: ${err.message || err}`);
          }
        }
      }
      
      draggedNode = null;
      snappedTarget = null;
      isDraggingNode = false;
      if (simulation) simulation.alphaTarget(0);
    }
  }

  // Zoom wheel
  function onWheel(e: WheelEvent) {
    e.preventDefault();
    const zoomFactor = 1.05;
    const nextK = e.deltaY < 0 ? transform.k * zoomFactor : transform.k / zoomFactor;
    transform.k = Math.max(0.2, Math.min(4, nextK));
    draw();
  }

  // Double click collapses / expands folders (isolated via stopPropagation)
  function onDoubleClick(e: MouseEvent) {
    e.stopPropagation();
    if (isDraggingNode) return; // Prevent collapse during drag cycles

    const coords = getMouseCoords(e);
    const clicked = nodes.find(n => {
      const dist = Math.hypot(n.x - coords.x, n.y - coords.y);
      return dist <= (n.data.is_folder ? (14 * 0.65) : (7 * 0.65));
    });

    if (clicked && clicked.data.is_folder) {
      const path = clicked.data.file_path;
      if (collapsedPaths.has(path)) {
        collapsedPaths.delete(path);
      } else {
        collapsedPaths.add(path);
      }
      reloadGraph();
    }
  }

  // Sprout (Right click context menu)
  function onContextMenu(e: MouseEvent) {
    e.preventDefault();
    const coords = getMouseCoords(e);
    const clicked = nodes.find(n => {
      const dist = Math.hypot(n.x - coords.x, n.y - coords.y);
      return dist <= (n.data.is_folder ? (14 * 0.65) : (7 * 0.65));
    });

    if (clicked && clicked.data.is_folder) {
      rightClickedNode = clicked;
      sproutDialog = {
        parentPath: clicked.data.file_path,
        x: e.clientX,
        y: e.clientY
      };
    }
  }

  async function handleSproutSubmit() {
    if (!sproutDialog || !sproutName.trim()) return;
    try {
      await ipc.sproutNode(sproutDialog.parentPath, sproutName.trim(), sproutType);
      sproutDialog = null;
      sproutName = '';
      await reloadGraph();
      await loadNotes();
    } catch (err: any) {
      alert(`Sprouting failed: ${err.message || err}`);
    }
  }

  // Pruning note command mapping cleanly to soft delete
  async function triggerPrune(node: any) {
    if (confirm(`Are you sure you want to delete and trash "${node.data.name}"?`)) {
      try {
        await ipc.pruneNode(node.data.file_path);
        hoveredNode = null;
        await reloadGraph();
        await loadNotes();
      } catch (err: any) {
        alert(`Deletion failed: ${err.message || err}`);
      }
    }
  }

  // Keyboard shortcut listener for micro-search finder
  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === '/') {
      e.preventDefault();
      showSearch = true;
      setTimeout(() => {
        searchInput?.focus();
      }, 50);
    } else if (e.key === 'Escape') {
      showSearch = false;
      searchQuery = '';
      draw();
    } else if (e.key === ' ' && hoveredNode && !hoveredNode.data.is_folder) {
      e.preventDefault();
      openSplitNote(hoveredNode.data.id);
    }
  }

  // Open note in side-split editor panel
  async function openSplitNote(noteId: string) {
    try {
      const note = await ipc.getNote(noteId);
      activeSplitNote = note;
    } catch (err) {
      console.error(err);
    }
  }

  async function handleEditorSave() {
    if (!activeSplitNote) return;
    try {
      await ipc.updateNote(
        activeSplitNote.id,
        activeSplitNote.title,
        activeSplitNote.body,
        activeSplitNote.parent_id,
        activeSplitNote.color,
        activeSplitNote.pinned,
        activeSplitNote.tags,
        true,
        'Manual'
      );
      await loadNotes();
    } catch (err) {
      console.error("Save failed:", err);
    }
  }

  // Close split view and expand the canvas back to full screen
  function handleEditorClose() {
    activeSplitNote = null;
  }

  let unlistenTrace: any = null;

  onMount(async () => {
    await reloadGraph();
    initSimulation();
    handleResize();

    unlistenTrace = await listen("mcp_trace", (event: any) => {
      const payload = event.payload as { note_id: string; action: string };
      triggerTraceParticle(payload.note_id, payload.action);
    });

    window.addEventListener("keydown", handleKeyDown);
    if (canvas && canvas.parentElement) {
      observer = new ResizeObserver(() => {
        handleResize();
      });
      observer.observe(canvas.parentElement);
    }

    const unsubscribe = salixActive.subscribe((val) => {
      if (val) {
        if (simulation) simulation.stop();
        isZenTransitioning = true;
        setTimeout(() => {
          isZenTransitioning = false;
          handleResize();
          if (simulation) simulation.alpha(0.3).restart();
        }, 300);
      }
    });

    return () => {
      unsubscribe();
    };
  });

  onDestroy(() => {
    if (unlistenTrace) unlistenTrace();
    window.removeEventListener("keydown", handleKeyDown);
    if (observer) observer.disconnect();
    if (simulation) simulation.stop();
  });
</script>

<div class="salix-layout">
  <div class="salix-pane" style="flex: {activeSplitNote ? '60%' : '100%'}">
    <!-- Obsidian-Style Minimalist Floating Control Overlay -->
    <div class="obsidian-physics-panel" class:expanded={showPhysicsPanel}>
      <button class="panel-toggle" onclick={() => showPhysicsPanel = !showPhysicsPanel}>
        ⚙️ Physics Controls {showPhysicsPanel ? '▼' : '▲'}
      </button>
      {#if showPhysicsPanel}
        <div class="panel-body">
          <div class="control-row">
            <label for="gravity">Gravity / Repulsion ({forceGravity})</label>
            <input id="gravity" type="range" min="-500" max="-10" bind:value={forceGravity} />
          </div>
          <div class="control-row">
            <label for="link-dist">Link Distance ({forceLinkDistance})</label>
            <input id="link-dist" type="range" min="20" max="200" bind:value={forceLinkDistance} />
          </div>
          <div class="control-row">
            <label for="collide-rad">Collision Radius ({forceCollisionRadius})</label>
            <input id="collide-rad" type="range" min="5" max="50" bind:value={forceCollisionRadius} />
          </div>
        </div>
      {/if}
    </div>

    <canvas
      bind:this={canvas}
      onmousedown={onMouseDown}
      onmousemove={onMouseMove}
      onmouseup={onMouseUp}
      onwheel={onWheel}
      ondblclick={onDoubleClick}
      oncontextmenu={onContextMenu}
    ></canvas>

    <!-- Maximization HUD overlay -->
    <div class="salix-hud">
      <span class="hud-tip">Press <strong>/</strong> to Filter · Hover + <strong>Space</strong> to Edit</span>
      {#if searchQuery}
        <span class="hud-search-indicator">Searching: {searchQuery}</span>
      {/if}
    </div>

    <!-- Micro search finder overlay -->
    {#if showSearch}
      <div class="micro-search">
        <input
          bind:this={searchInput}
          type="text"
          placeholder="Fuzzy filter nodes..."
          bind:value={searchQuery}
          oninput={draw}
        />
        <button onclick={() => { showSearch = false; searchQuery = ''; draw(); }}>×</button>
      </div>
    {/if}

    <!-- Sprout Context Dialog -->
    {#if sproutDialog}
      <div
        class="sprout-context-menu"
        style="top: {sproutDialog.y}px; left: {sproutDialog.x}px;"
      >
        <h4>Sprout New Tomurcuk</h4>
        <input type="text" bind:value={sproutName} placeholder="Name..." />
        <div class="type-selector">
          <label>
            <input type="radio" value="note" bind:group={sproutType} /> Note
          </label>
          <label>
            <input type="radio" value="folder" bind:group={sproutType} /> Folder
          </label>
        </div>
        <div class="actions">
          <button onclick={handleSproutSubmit} class="btn-primary">Sprout</button>
          <button onclick={() => sproutDialog = null} class="btn-cancel">Cancel</button>
        </div>
      </div>
    {/if}

    <!-- Hover details and Pruning Scissors HUD (Non-blocking layout) -->
    {#if hoveredNode}
      <div class="node-hover-details">
        <div class="hover-head">
          <span class="name">{hoveredNode.data.name}</span>
          {#if hoveredNode.data.is_folder}
            <span class="badge folder">Folder</span>
          {:else if hoveredNode.is_encrypted}
            <span class="badge encrypted">Encrypted</span>
          {:else}
            <span class="badge note">Note ({hoveredNode.char_size} chars)</span>
          {/if}
        </div>
        <div class="pruning-trigger">
          <button onclick={() => triggerPrune(hoveredNode)} class="btn-prune">
            🪓 Prune (Move to Trash)
          </button>
        </div>
      </div>
    {/if}
  </div>

  <!-- Vertical text editor panel from the right -->
  {#if activeSplitNote}
    <div class="split-editor-panel">
      <div class="editor-header">
        <h3>{activeSplitNote.title}</h3>
        <div class="header-actions">
          <button onclick={handleEditorSave} title="Save Note" class="btn-save">💾 Save</button>
          <button onclick={handleEditorClose} title="Close Split View" class="btn-close">Collapse</button>
        </div>
      </div>
      <div class="editor-body">
        <textarea
          bind:value={activeSplitNote.body}
          placeholder="Write markdown here..."
        ></textarea>
      </div>
    </div>
  {/if}
</div>

<style>
  .salix-layout {
    display: flex;
    width: 100%;
    height: 100%;
    overflow: hidden;
    position: relative;
    background-color: #1c1c1e;
  }

  .salix-pane {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    transition: flex 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  canvas {
    width: 100%;
    height: 100%;
    display: block;
    cursor: grab;
  }

  canvas:active {
    cursor: grabbing;
  }

  /* Obsidian-Style Minimalist Floating Control Overlay */
  .obsidian-physics-panel {
    position: absolute;
    top: 12px;
    left: 12px;
    z-index: 100;
    background: rgba(28, 28, 30, 0.85);
    backdrop-filter: blur(15px);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 8px;
    width: 200px;
    color: #e1e1e6;
    font-family: system-ui;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
    pointer-events: auto;
  }

  .panel-toggle {
    width: 100%;
    background: transparent;
    border: none;
    color: #0a84ff;
    font-size: 11px;
    font-weight: 700;
    text-align: left;
    cursor: pointer;
    outline: none;
    display: flex;
    justify-content: space-between;
  }

  .panel-body {
    margin-top: 8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .control-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .control-row label {
    font-size: 10px;
    color: #aeaeae;
  }

  .control-row input[type="range"] {
    width: 100%;
    height: 4px;
    border-radius: 2px;
    outline: none;
  }

  /* HUD Tip */
  .salix-hud {
    position: absolute;
    top: 12px;
    right: 230px; /* Offset to clear micro-search and physics panel */
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(10px);
    border: 1px solid rgba(255, 255, 255, 0.08);
    padding: 6px 12px;
    border-radius: 6px;
    color: #aeaeae;
    font-size: 11px;
    pointer-events: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .hud-tip strong {
    color: #0a84ff;
  }

  .hud-search-indicator {
    color: #30d158;
    font-weight: 600;
  }

  /* Micro search finder */
  .micro-search {
    position: absolute;
    top: 12px;
    right: 12px;
    background: rgba(28, 28, 30, 0.85);
    backdrop-filter: blur(15px);
    border: 1px solid rgba(255, 255, 255, 0.1);
    padding: 4px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    gap: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
  }

  .micro-search input {
    background: transparent;
    border: none;
    color: #ffffff;
    font-size: 12px;
    outline: none;
    padding: 4px 6px;
    width: 160px;
  }

  .micro-search button {
    background: transparent;
    border: none;
    color: #8e8e93;
    font-size: 16px;
    cursor: pointer;
    padding: 0 4px;
  }

  /* Context Menu */
  .sprout-context-menu {
    position: fixed;
    background: #2c2c2e;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 8px;
    padding: 10px;
    width: 180px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.6);
    z-index: 1000;
    font-family: system-ui;
  }

  .sprout-context-menu h4 {
    margin: 0 0 8px 0;
    font-size: 11px;
    color: #aeaeae;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .sprout-context-menu input[type="text"] {
    width: 100%;
    box-sizing: border-box;
    background: #1c1c1e;
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: white;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 12px;
    margin-bottom: 8px;
    outline: none;
  }

  .type-selector {
    display: flex;
    gap: 12px;
    margin-bottom: 8px;
    font-size: 11px;
    color: #d1d1d6;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }

  .actions button {
    font-size: 11px;
    padding: 4px 8px;
    border-radius: 4px;
    border: none;
    cursor: pointer;
  }

  .btn-primary {
    background: #0a84ff;
    color: white;
  }

  .btn-cancel {
    background: #3a3a3c;
    color: #d1d1d6;
  }

  /* Node Hover details - Non-blocking bottom-left layout */
  .node-hover-details {
    position: absolute;
    bottom: 12px;
    left: 12px;
    z-index: 90;
    background: rgba(28, 28, 30, 0.9);
    backdrop-filter: blur(12px);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 8px 12px;
    color: white;
    font-family: system-ui;
    display: flex;
    flex-direction: column;
    gap: 8px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
    pointer-events: auto;
  }

  .hover-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .hover-head .name {
    font-weight: 600;
    font-size: 13px;
  }

  .badge {
    font-size: 9px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 10px;
    text-transform: uppercase;
  }

  .badge.folder { background: #0a84ff; color: white; }
  .badge.note { background: #30d158; color: black; }
  .badge.encrypted { background: #ff453a; color: white; }

  .btn-prune {
    background: rgba(255, 69, 58, 0.15);
    border: 1px solid rgba(255, 69, 58, 0.3);
    color: #ff453a;
    font-size: 10px;
    padding: 4px 8px;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 600;
  }

  .btn-prune:hover {
    background: #ff453a;
    color: white;
  }

  /* Split Editor Panel */
  .split-editor-panel {
    flex: 40%;
    height: 100%;
    background-color: #1e1e1e;
    border-left: 1px solid rgba(255, 255, 255, 0.08);
    display: flex;
    flex-direction: column;
    transition: flex 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .editor-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    background-color: #252526;
  }

  .editor-header h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: #e1e1e6;
  }

  .header-actions {
    display: flex;
    gap: 8px;
  }

  .header-actions button {
    font-size: 11px;
    padding: 4px 8px;
    border-radius: 4px;
    border: none;
    cursor: pointer;
    font-weight: 600;
  }

  .btn-save {
    background: #30d158;
    color: black;
  }

  .btn-close {
    background: #3a3a3c;
    color: #e1e1e6;
  }

  .editor-body {
    flex: 1;
    padding: 12px;
  }

  .editor-body textarea {
    width: 100%;
    height: 100%;
    background: transparent;
    border: none;
    resize: none;
    outline: none;
    color: #e1e1e6;
    font-family: Menlo, Monaco, Consolas, monospace;
    font-size: 13px;
    line-height: 1.6;
  }
</style>
