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
  let forceLinkDistance = $state(45);
  let forceCollisionRadius = $state(40);
  let showPhysicsPanel = $state(false);
  let lodThreshold = $state(1.5); // Adjustable zoom text visibility limit (0.1 to 2.0)

  // Mouse interaction
  let transform = $state({ x: 0, y: 0, k: 1 });
  let draggedNode = $state<any | null>(null);
  let dragStartMouse = { x: 0, y: 0 };
  let dragStartNodePos = { x: 0, y: 0 };
  let hoveredNode = $state<any | null>(null);
  let hoveredLink = $state<any | null>(null);
  let rightClickedNode = $state<any | null>(null);
  
  // Snap Target for Drag & Drop
  let snappedTarget = $state<any | null>(null);

  // Dynamic Neon Particles for Telemetry
  let particles = $state<any[]>([]);

  // Active glowing paths map for neon telemetry (whole network glow)
  let glowingPaths = new Map<string, { nodesPath: string[], intensity: number }>();

  // Continuous animation frame hook for telemetry when simulation is cold
  let animFrameId: number | null = null;

  // Listen to Tauri resize and layout transitions
  let observer: ResizeObserver | null = null;
  let isZenTransitioning = $state(false);

  // Helper functions for hovering sub-tree focus
  function getDescendants(nodeId: string): Set<string> {
    const descendants = new Set<string>();
    const queue = [nodeId];
    while (queue.length > 0) {
      const currId = queue.shift();
      if (!currId) continue;
      descendants.add(currId);
      links.forEach(l => {
        if (l.source.id === currId && !descendants.has(l.target.id)) {
          queue.push(l.target.id);
        }
      });
    }
    return descendants;
  }

  function getAncestors(nodeId: string): Set<string> {
    const ancestors = new Set<string>();
    let currentId = nodeId;
    while (true) {
      const link = links.find(l => l.target.id === currentId);
      if (link && link.source.id !== currentId) {
        ancestors.add(link.source.id);
        currentId = link.source.id;
      } else {
        break;
      }
    }
    return ancestors;
  }

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
        const baseRadius = d.data.is_folder ? 22 : 12;
        return baseRadius * 0.65 * 0.6;
      }))
      .force("x", d3Force.forceX((d: any) => d.targetX).strength(0.4))
      .force("y", d3Force.forceY((d: any) => d.targetY).strength(0.4))
      .on("tick", draw);
  }

  // Calculate distance from point P to line segment AB
  function getDistanceToSegment(px: number, py: number, x1: number, y1: number, x2: number, y2: number) {
    const dx = x2 - x1;
    const dy = y2 - y1;
    if (dx === 0 && dy === 0) return Math.hypot(px - x1, py - y1);
    const t = Math.max(0, Math.min(1, ((px - x1) * dx + (py - y1) * dy) / (dx * dx + dy * dy)));
    return Math.hypot(px - (x1 + t * dx), py - (y1 + t * dy));
  }

  // Draw loop
  function draw() {
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // Dynamic CSS Variable Extraction (No hardcoded values)
    const style = window.getComputedStyle(canvas);
    const themeAccent = style.getPropertyValue("--graph-node-accent") || "#0a84ff";
    const themeText = style.getPropertyValue("--text-primary") || "#ffffff";
    const themeTextSecondary = style.getPropertyValue("--text-secondary") || "rgba(255, 255, 255, 0.75)";
    
    const telemetryGlow = style.getPropertyValue("--telemetry-glow") || "#64d2ff";
    
    const graphNodeBorder = style.getPropertyValue("--graph-node-border") || "#ffffff";
    const graphFolderCollapsed = style.getPropertyValue("--graph-folder-collapsed") || "#ff9f0a";
    const graphNodeEncrypted = style.getPropertyValue("--graph-node-encrypted") || "#ff453a";
    const graphNodeActive = style.getPropertyValue("--graph-node-active") || "#30d158";
    const graphSnapHighlight = style.getPropertyValue("--graph-snap-highlight") || "#00ffff";

    ctx.save();
    ctx.clearRect(0, 0, width, height);
    
    // Grid background
    drawGrid(ctx, style);

    ctx.translate(width / 2 + transform.x, height / 2 + transform.y);
    ctx.scale(transform.k, transform.k);

    // Hover-Focus Networking Highlight State
    let focusedNodeIds = new Set<string>();
    let hasFocusState = false;

    if (hoveredNode) {
      hasFocusState = true;
      focusedNodeIds = new Set([
        hoveredNode.id,
        ...getAncestors(hoveredNode.id),
        ...getDescendants(hoveredNode.id)
      ]);
    } else if (hoveredLink) {
      hasFocusState = true;
      const targetId = hoveredLink.target.id;
      focusedNodeIds = new Set([
        targetId,
        hoveredLink.source.id,
        ...getAncestors(targetId),
        ...getDescendants(targetId)
      ]);
    }

    // Filter logic
    const hasQuery = searchQuery.trim().length > 0;
    const lowerQuery = searchQuery.toLowerCase();

    // Draw Links
    links.forEach(l => {
      let opacity = 0.4;
      if (hasFocusState) {
        opacity = (focusedNodeIds.has(l.source.id) && focusedNodeIds.has(l.target.id)) ? 0.9 : 0.08;
      } else if (hasQuery) {
        const sourceMatch = l.source.data.name.toLowerCase().includes(lowerQuery);
        const targetMatch = l.target.data.name.toLowerCase().includes(lowerQuery);
        opacity = (sourceMatch || targetMatch) ? 0.9 : 0.15;
      }
      ctx.beginPath();
      ctx.strokeStyle = themeAccent;
      ctx.globalAlpha = opacity;
      ctx.lineWidth = 1.0;
      ctx.moveTo(l.source.x, l.source.y);
      ctx.lineTo(l.target.x, l.target.y);
      ctx.stroke();
      ctx.globalAlpha = 1.0;
    });

    // Draw Static Glowing Chain (Cybernetic Accent Glow)
    glowingPaths.forEach((pathObj, key) => {
      ctx.save();
      ctx.strokeStyle = telemetryGlow;
      ctx.shadowBlur = 20;
      ctx.shadowColor = telemetryGlow;
      ctx.lineWidth = 3.0;
      ctx.globalAlpha = pathObj.intensity;
      ctx.beginPath();
      for (let i = 0; i < pathObj.nodesPath.length - 1; i++) {
        const nodeA = nodes.find(n => n.id === pathObj.nodesPath[i]);
        const nodeB = nodes.find(n => n.id === pathObj.nodesPath[i + 1]);
        if (nodeA && nodeB) {
          ctx.moveTo(nodeA.x, nodeA.y);
          ctx.lineTo(nodeB.x, nodeB.y);
        }
      }
      ctx.stroke();
      
      // Node glowing rings
      pathObj.nodesPath.forEach(nid => {
        const n = nodes.find(x => x.id === nid);
        if (n) {
          ctx.beginPath();
          const r = n.data.is_folder ? (14 * 0.65 * 0.6) : (7 * 0.65 * 0.6);
          ctx.arc(n.x, n.y, r + 2, 0, 2 * Math.PI);
          ctx.fillStyle = telemetryGlow;
          ctx.globalAlpha = pathObj.intensity * 0.4;
          ctx.fill();
        }
      });
      ctx.restore();

      pathObj.intensity -= 0.008;
      if (pathObj.intensity <= 0) {
        glowingPaths.delete(key);
      }
    });

    // Draw Nodes
    nodes.forEach(n => {
      const isFolder = n.data.is_folder;
      const isEncrypted = n.is_encrypted;
      
      let opacity = 1.0;
      let glow = false;
      if (hasFocusState) {
        opacity = focusedNodeIds.has(n.id) ? 1.0 : 0.15;
      } else if (hasQuery) {
        const match = n.data.name.toLowerCase().includes(lowerQuery);
        opacity = match ? 1.0 : 0.15;
        glow = match;
      }

      ctx.save();
      ctx.globalAlpha = opacity;

      // Glow effect if matched
      if (glow) {
        ctx.shadowBlur = 15;
        ctx.shadowColor = graphNodeActive;
      }

      // Draw node shape
      ctx.beginPath();
      const radius = isFolder ? (14 * 0.65 * 0.6) : (7 * 0.65 * 0.6);
      ctx.arc(n.x, n.y, radius, 0, 2 * Math.PI);

      if (isFolder) {
        ctx.fillStyle = collapsedPaths.has(n.data.file_path) ? graphFolderCollapsed : themeAccent;
      } else if (isEncrypted) {
        ctx.fillStyle = graphNodeEncrypted;
      } else {
        ctx.fillStyle = graphNodeActive;
      }

      ctx.fill();
      ctx.strokeStyle = graphNodeBorder;
      ctx.lineWidth = 0.5;
      ctx.stroke();

      if (isEncrypted) {
        ctx.fillStyle = graphNodeBorder;
        ctx.font = "bold 5px system-ui";
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
      }

      // Node label - Zoom Dependent LOD text blackout
      if (transform.k >= lodThreshold) {
        ctx.shadowBlur = 0;
        ctx.fillStyle = (hoveredNode === n) ? themeText : themeTextSecondary;
        ctx.font = "9px var(--font-sans, system-ui)";
        ctx.textAlign = "center";
        ctx.fillText(n.data.name, n.x, n.y + radius + 9);
      }

      ctx.restore();
    });

    // Snapping logic visual overlay during drag
    if (draggedNode && snappedTarget) {
      ctx.save();
      ctx.beginPath();
      ctx.arc(snappedTarget.x, snappedTarget.y, (14 * 0.65 * 0.6) + 4, 0, 2 * Math.PI);
      ctx.strokeStyle = graphSnapHighlight;
      ctx.lineWidth = 2.0;
      ctx.shadowBlur = 12;
      ctx.shadowColor = graphSnapHighlight;
      ctx.stroke();
      ctx.restore();
    }

    // Draw Dynamic Neon Telemetry Particles
    updateAndDrawParticles(ctx, style);

    ctx.restore();
  }

  function drawGrid(ctx: CanvasRenderingContext2D, style: CSSStyleDeclaration) {
    const gridColor = style.getPropertyValue("--graph-grid") || "rgba(255, 255, 255, 0.02)";
    ctx.strokeStyle = gridColor;
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

  function updateAndDrawParticles(ctx: CanvasRenderingContext2D, style: CSSStyleDeclaration) {
    const activeParticles: any[] = [];
    particles.forEach(p => {
      p.progress += p.speed;
      if (p.progress < 1.0) {
        const segmentCount = p.pathNodes.length - 1;
        const rawIdx = p.progress * segmentCount;
        const idx = Math.min(segmentCount - 1, Math.floor(rawIdx));
        const segmentT = rawIdx - idx;
        const nodeA = p.pathNodes[idx];
        const nodeB = p.pathNodes[idx + 1];

        if (nodeA && nodeB) {
          const x = nodeA.x + (nodeB.x - nodeA.x) * segmentT;
          const y = nodeA.y + (nodeB.y - nodeA.y) * segmentT;
          
          ctx.save();
          ctx.beginPath();
          ctx.arc(x, y, 4.5, 0, 2 * Math.PI);
          
          const themeAccent = style.getPropertyValue("--graph-node-accent") || "#0a84ff";
          const pColor = style.getPropertyValue(p.colorVar) || themeAccent;
          ctx.fillStyle = pColor;
          ctx.shadowBlur = 15;
          ctx.shadowColor = pColor;
          ctx.fill();
          ctx.restore();
        }
        activeParticles.push(p);
      }
    });
    particles = activeParticles;
  }

  // Animation frame loop to redraw when simulation is cold
  function animationLoop() {
    if (particles.length > 0 || glowingPaths.size > 0) {
      draw();
      animFrameId = requestAnimationFrame(animationLoop);
    } else {
      animFrameId = null;
    }
  }

  // Triggers path telemetry glow + dynamic particles at the same time
  function triggerTraceParticle(noteId: string, action: string) {
    if (!d3Root) return;

    let foundHierNode: any = null;
    d3Root.descendants().forEach((d: any) => {
      const id = d.data.id || d.data.file_path;
      if (id === noteId) {
        foundHierNode = d;
      }
    });

    if (!foundHierNode) return;

    const path: any[] = [];
    let curr = foundHierNode;
    while (curr) {
      const id = curr.data.id || curr.data.file_path;
      const physicsNode = nodes.find(n => n.id === id);
      if (physicsNode) {
        path.push(physicsNode);
      }
      curr = curr.parent;
    }

    if (path.length === 0) return;

    // Static Cyan Neon chain glow
    const pathIds = path.map(x => x.id);
    glowingPaths.set(noteId, {
      nodesPath: pathIds,
      intensity: 1.0
    });

    // Particle directional flow - map variables to CSS variables in app.css
    const pathNodes = action === "read" ? path : [...path].reverse();
    const colorVar = action === "read" ? "--telemetry-read" : "--telemetry-write";

    particles.push({
      pathNodes,
      progress: 0.0,
      speed: 0.018,
      colorVar
    });

    // Start rendering loops if not already active
    if (!animFrameId) {
      animFrameId = requestAnimationFrame(animationLoop);
    }
  }

  // Handle Resize
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
    if (e.button === 2) return; // Context menu handled in onContextMenu
    const coords = getMouseCoords(e);
    
    // Check if clicked node
    const clicked = nodes.find(n => {
      const dist = Math.hypot(n.x - coords.x, n.y - coords.y);
      return dist <= (n.data.is_folder ? (14 * 0.65 * 0.6) : (7 * 0.65 * 0.6));
    });

    if (clicked) {
      draggedNode = clicked;
      isDraggingNode = false;
      dragStartMouse = { x: e.clientX, y: e.clientY };
      dragStartNodePos = { x: clicked.x, y: clicked.y };
      if (simulation) simulation.alphaTarget(0.3).restart();
    } else {
      dragStartMouse = { x: e.clientX, y: e.clientY };
    }
  }

  function onMouseMove(e: MouseEvent) {
    const coords = getMouseCoords(e);

    // Hover check for Nodes
    hoveredNode = nodes.find(n => {
      const dist = Math.hypot(n.x - coords.x, n.y - coords.y);
      return dist <= (n.data.is_folder ? (14 * 0.65 * 0.6) : (7 * 0.65 * 0.6));
    }) || null;

    // Hover check for Links (only if no node is hovered)
    if (hoveredNode) {
      hoveredLink = null;
    } else {
      hoveredLink = links.find(l => {
        const dist = getDistanceToSegment(coords.x, coords.y, l.source.x, l.source.y, l.target.x, l.target.y);
        return dist < 8.0;
      }) || null;
    }

    if (draggedNode) {
      isDraggingNode = true;
      const dx = (e.clientX - dragStartMouse.x) / transform.k;
      const dy = (e.clientY - dragStartMouse.y) / transform.k;
      const proposedX = dragStartNodePos.x + dx;
      const proposedY = dragStartNodePos.y + dy;

      // Magnetic snapping anchor mechanism (Distance < 20px)
      snappedTarget = null;
      for (const n of nodes) {
        if (n !== draggedNode && n.data.is_folder) {
          const dist = Math.hypot(n.x - proposedX, n.y - proposedY);
          if (dist < 20) {
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
      transform.x += e.clientX - dragStartMouse.x;
      transform.y += e.clientY - dragStartMouse.y;
      dragStartMouse = { x: e.clientX, y: e.clientY };
      draw();
    }
    draw();
  }

  async function onMouseUp(e: MouseEvent) {
    if (draggedNode) {
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
    if (isDraggingNode) return;

    const coords = getMouseCoords(e);
    const clicked = nodes.find(n => {
      const dist = Math.hypot(n.x - coords.x, n.y - coords.y);
      return dist <= (n.data.is_folder ? (14 * 0.65 * 0.6) : (7 * 0.65 * 0.6));
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

  // Context Menu and Link-Level Edge Interception for Budama (Strict Target Isolation & Native Ask Dialog)
  async function onContextMenu(e: MouseEvent) {
    e.preventDefault();
    const coords = getMouseCoords(e);

    // 1. STRICT NODE ISOLATION: Check if click intersects with any Node first
    const clickedNode = nodes.find(n => {
      const dist = Math.hypot(n.x - coords.x, n.y - coords.y);
      return dist <= (n.data.is_folder ? (14 * 0.65 * 0.6) : (7 * 0.65 * 0.6));
    });

    if (clickedNode) {
      if (clickedNode.data.is_folder) {
        rightClickedNode = clickedNode;
        sproutDialog = {
          parentPath: clickedNode.data.file_path,
          x: e.clientX,
          y: e.clientY
        };
      }
      return;
    }

    // 2. LINK-LEVEL INTERCEPTION: Only check links if no node was right-clicked
    let closestLink: any = null;
    let minDistance = Infinity;
    links.forEach(l => {
      const dist = getDistanceToSegment(coords.x, coords.y, l.source.x, l.source.y, l.target.x, l.target.y);
      if (dist < minDistance) {
        minDistance = dist;
        closestLink = l;
      }
    });

    if (minDistance < 8.0) {
      // Confirmed right-click on link edge. Prompt native OS dialog box.
      const downstreamNode = closestLink.target;
      
      try {
        const { ask } = await import('@tauri-apps/plugin-dialog');
        const yes = await ask(
          "Emin misin? Bu dalı budamak, altındaki tüm klasör ve notları kalıcı olarak Noda Çöp Kutusu'na taşıyacaktır.",
          { title: 'Budama Onayı', kind: 'warning' }
        );
        if (yes) {
          await ipc.pruneNode(downstreamNode.data.file_path);
          hoveredNode = null;
          hoveredLink = null;
          await reloadGraph();
          await loadNotes();
        }
      } catch (err: any) {
        console.error("Native dialog import failed, falling back to window.confirm:", err);
        if (confirm("Emin misin? Bu dalı budamak, altındaki tüm klasör ve notları kalıcı olarak Noda Çöp Kutusu'na taşıyacaktır.")) {
          await ipc.pruneNode(downstreamNode.data.file_path);
          hoveredNode = null;
          hoveredLink = null;
          await reloadGraph();
          await loadNotes();
        }
      }
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
    if (animFrameId) cancelAnimationFrame(animFrameId);
  });
</script>

<div class="salix-layout">
  <div class="salix-pane" style="flex: {activeSplitNote ? '60%' : '100%'}">
    <!-- Obsidian-Style Minimalist Floating Control Overlay -->
    <div class="obsidian-physics-panel" class:expanded={showPhysicsPanel}>
      <button class="panel-toggle" onclick={() => showPhysicsPanel = !showPhysicsPanel}>
        ⚙️ Physics & Filters {showPhysicsPanel ? '▼' : '▲'}
      </button>
      {#if showPhysicsPanel}
        <div class="panel-body">
          <div class="control-row">
            <label for="gravity">Gravity / Repulsion ({forceGravity})</label>
            <input 
              id="gravity" 
              type="range" 
              min="-500" 
              max="-10" 
              bind:value={forceGravity} 
              oninput={(e: any) => {
                forceGravity = parseFloat(e.target.value);
                if (simulation) {
                  simulation.force("charge").strength(forceGravity);
                  simulation.alphaTarget(0.3).restart();
                }
              }}
            />
          </div>
          <div class="control-row">
            <label for="link-dist">Link Distance ({forceLinkDistance})</label>
            <input 
              id="link-dist" 
              type="range" 
              min="20" 
              max="200" 
              bind:value={forceLinkDistance} 
              oninput={(e: any) => {
                forceLinkDistance = parseFloat(e.target.value);
                if (simulation) {
                  simulation.force("link").distance(forceLinkDistance);
                  simulation.alphaTarget(0.3).restart();
                }
              }}
            />
          </div>
          <div class="control-row">
            <label for="collide-rad">Collision Radius ({forceCollisionRadius})</label>
            <input 
              id="collide-rad" 
              type="range" 
              min="5" 
              max="50" 
              bind:value={forceCollisionRadius} 
              oninput={(e: any) => {
                forceCollisionRadius = parseFloat(e.target.value);
                if (simulation) {
                  simulation.force("collide").radius((d: any) => {
                    return d.data.is_folder ? forceCollisionRadius * 1.83 * 0.65 * 0.6 : forceCollisionRadius * 0.65 * 0.6;
                  });
                  simulation.alphaTarget(0.3).restart();
                }
              }}
            />
          </div>
          <div class="control-row">
            <label for="lod-thresh">Text Filter Threshold ({lodThreshold})</label>
            <input 
              id="lod-thresh" 
              type="range" 
              min="0.1" 
              max="2.0" 
              step="0.05" 
              bind:value={lodThreshold} 
              oninput={draw}
            />
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
      <span class="hud-tip">Right-click Link to Budama · Press <strong>/</strong> to Filter · Hover + <strong>Space</strong> to Edit</span>
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
    background-color: var(--graph-bg);
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
    left: 300px;
    z-index: 100;
    background: var(--toolbar-bg);
    backdrop-filter: blur(15px);
    border: 1px solid var(--border-normal);
    border-radius: 6px;
    padding: 8px;
    width: 200px;
    color: var(--text-primary);
    font-family: var(--font-sans);
    box-shadow: var(--shadow-md);
    pointer-events: auto;
  }

  .panel-toggle {
    width: 100%;
    background: transparent;
    border: none;
    color: var(--accent);
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
    color: var(--text-secondary);
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
    background: var(--overlay-bg);
    backdrop-filter: blur(10px);
    border: 1px solid var(--border-subtle);
    padding: 6px 12px;
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 11px;
    pointer-events: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .hud-tip strong {
    color: var(--accent);
  }

  .hud-search-indicator {
    color: var(--color-green);
    font-weight: 600;
  }

  /* Micro search finder */
  .micro-search {
    position: absolute;
    top: 12px;
    right: 12px;
    background: var(--toolbar-bg);
    backdrop-filter: blur(15px);
    border: 1px solid var(--border-normal);
    padding: 4px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    gap: 6px;
    box-shadow: var(--shadow-md);
  }

  .micro-search input {
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 12px;
    outline: none;
    padding: 4px 6px;
    width: 160px;
  }

  .micro-search button {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 16px;
    cursor: pointer;
    padding: 0 4px;
  }

  /* Context Menu */
  .sprout-context-menu {
    position: fixed;
    background: var(--bg-elevated);
    border: 1px solid var(--border-normal);
    border-radius: 8px;
    padding: 10px;
    width: 180px;
    box-shadow: var(--shadow-lg);
    z-index: 1000;
    font-family: var(--font-sans);
  }

  .sprout-context-menu h4 {
    margin: 0 0 8px 0;
    font-size: 11px;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .sprout-context-menu input[type="text"] {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-window);
    border: 1px solid var(--border-subtle);
    color: var(--text-primary);
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
    color: var(--text-secondary);
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
    background: var(--accent);
    color: var(--text-primary);
  }

  .btn-cancel {
    background: var(--bg-elevated-2);
    color: var(--text-secondary);
  }

  /* Split Editor Panel */
  .split-editor-panel {
    flex: 40%;
    height: 100%;
    background-color: var(--bg-editor);
    border-left: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    transition: flex 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .editor-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border-subtle);
    background-color: var(--bg-elevated);
  }

  .editor-header h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
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
    background: var(--color-green);
    color: var(--bg-window);
  }

  .btn-close {
    background: var(--bg-elevated-2);
    color: var(--text-primary);
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
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 1.6;
  }
</style>
