(() => {
  const root = document.documentElement;
  const theme = document.querySelector("#theme");
  const preference = window.matchMedia("(prefers-color-scheme: dark)");
  const applyTheme = (value) => {
    root.dataset.theme = value;
    theme.textContent = value === "dark" ? "☀" : "☾";
    theme.setAttribute(
      "aria-label",
      `Switch to ${value === "dark" ? "light" : "dark"} theme`,
    );
  };
  let saved;
  try {
    saved = localStorage.getItem("containerdesk-docs-theme");
  } catch {
    /* Storage is optional. */
  }
  applyTheme(
    saved === "light" || saved === "dark"
      ? saved
      : preference.matches
        ? "dark"
        : "light",
  );
  theme.hidden = false;
  theme.addEventListener("click", () => {
    saved = root.dataset.theme === "dark" ? "light" : "dark";
    applyTheme(saved);
    try {
      localStorage.setItem("containerdesk-docs-theme", saved);
    } catch {
      /* Keep the current page usable. */
    }
  });
  preference.addEventListener("change", () => {
    if (!saved) applyTheme(preference.matches ? "dark" : "light");
  });
  const menu = document.querySelector("#menu");
  menu.hidden = false;
  const closeMenu = () => {
    root.classList.remove("nav-open");
    menu.setAttribute("aria-expanded", "false");
  };
  menu.addEventListener("click", () => {
    const open = root.classList.toggle("nav-open");
    menu.setAttribute("aria-expanded", String(open));
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && root.classList.contains("nav-open")) {
      closeMenu();
      menu.focus();
    }
  });
  document
    .querySelectorAll("#sidebar a")
    .forEach((link) => link.addEventListener("click", closeMenu));
  const search = document.querySelector("#search");
  document.querySelector("#search-box").hidden = false;
  const groups = [...document.querySelectorAll(".nav-group")];
  let priorOpen = null;
  search.addEventListener("input", () => {
    const query = search.value.trim().toLocaleLowerCase();
    if (query && !priorOpen) priorOpen = groups.map((group) => group.open);
    let count = 0;
    groups.forEach((group, index) => {
      let matches = 0;
      group.querySelectorAll("li").forEach((item) => {
        item.hidden = !item.textContent.toLocaleLowerCase().includes(query);
        if (!item.hidden) {
          matches++;
          count++;
        }
      });
      group.hidden = matches === 0;
      if (query) group.open = true;
      else if (priorOpen) group.open = priorOpen[index];
    });
    if (!query) priorOpen = null;
    document.querySelector("#search-status").textContent = query
      ? `${count} matching guides`
      : "";
  });
  const toc = document.querySelector("#toc");
  document.querySelectorAll(".prose h2[id]").forEach((heading) => {
    const item = document.createElement("li");
    const link = document.createElement("a");
    link.href = `#${heading.id}`;
    link.textContent = heading.textContent;
    item.append(link);
    toc.querySelector("ul").append(item);
  });
  toc.hidden = !toc.querySelector("li");
  document.querySelectorAll(".prose table").forEach((table) => {
    const wrapper = document.createElement("div");
    wrapper.className = "table-scroll";
    wrapper.tabIndex = 0;
    wrapper.setAttribute("role", "region");
    wrapper.setAttribute("aria-label", "Scrollable table");
    table.before(wrapper);
    wrapper.append(table);
  });
  root.classList.add("js-ready");
})();
