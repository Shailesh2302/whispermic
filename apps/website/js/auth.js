// Simple auth (demo) + ready for real backend.
// Set API_URL in config.js to plug Supabase / your REST API.
const Auth = {
  key: "whispermic_user",
  get() { try { return JSON.parse(localStorage.getItem(this.key)); } catch { return null; } },
  set(u) { localStorage.setItem(this.key, JSON.stringify(u)); },
  clear() { localStorage.removeItem(this.key); },
};

async function apiSignup(name, email, password) {
  if (window.APP_CONFIG?.API_URL) {
    const r = await fetch(window.APP_CONFIG.API_URL + "/auth/signup", {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name, email, password }),
    });
    if (!r.ok) throw new Error("Signup failed on server");
    return r.json();
  }
  // demo fallback
  await new Promise(r => setTimeout(r, 600));
  const users = JSON.parse(localStorage.getItem("whispermic_users") || "[]");
  if (users.find(u => u.email === email)) throw new Error("Email already registered. Try login.");
  const user = { name, email, createdAt: new Date().toISOString() };
  users.push({ ...user, password: btoa(password) });
  localStorage.setItem("whispermic_users", JSON.stringify(users));
  return user;
}

async function apiLogin(email, password) {
  if (window.APP_CONFIG?.API_URL) {
    const r = await fetch(window.APP_CONFIG.API_URL + "/auth/login", {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email, password }),
    });
    if (!r.ok) throw new Error("Invalid email or password");
    return r.json();
  }
  await new Promise(r => setTimeout(r, 600));
  const users = JSON.parse(localStorage.getItem("whispermic_users") || "[]");
  const u = users.find(u => u.email === email && u.password === btoa(password));
  if (!u) throw new Error("Invalid email or password (demo: signup first).");
  return { name: u.name, email: u.email };
}

function showErr(id, msg) { const e = document.getElementById(id); if (e) { e.textContent = msg; e.style.display = "block"; } }
function hideErr(id) { const e = document.getElementById(id); if (e) e.style.display = "none"; }
function showOk(id, msg) { const e = document.getElementById(id); if (e) { e.textContent = msg; e.style.display = "block"; } }

function updateNav() {
  const user = Auth.get();
  document.querySelectorAll("[data-auth-area]").forEach(el => {
    if (user) {
      el.innerHTML = `<span style="color:var(--mut);font-size:13px">Hi, ${user.name || user.email}</span>
        <button class="btn small ghost" onclick="logout()">Logout</button>`;
    }
  });
}
function logout() { Auth.clear(); location.href = "index.html"; }
document.addEventListener("DOMContentLoaded", updateNav);
