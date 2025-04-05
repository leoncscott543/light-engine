
# 🎼 ARIA — The Declarative Language of Light

**ARIA** is the domain-specific language (DSL) of the Light Engine.  
It is the voice of physical reality — a clear, structured way to define matter, behavior, time, energy, and environment from first principles.

ARIA is not a scripting language. It’s not a markup language.  
It is a **semantic scaffold for nature** — letting you compose the universe one particle, one interaction, and one cause at a time.

---

## ✨ Philosophy

Just as musical arias express emotion through precise structure and flow, **ARIA** expresses **reality** through **declarative physical truth**.

It was designed for:

- Scientists who want simulation without illusion
- Engineers who want control without chaos
- Designers who want beauty without faking
- Dreamers who want to build new worlds from particles upward

ARIA speaks in **particles**, **materials**, **environments**, and **causality** — not textures, prefabs, or hacks.

---

## 📐 Syntax Goals

- **Human-readable**
- **Declarative and hierarchical**
- **Composable**
- **Statically analyzable**
- **Zero hidden behavior**

Example (futuristic preview):

```aria
thing Human {
    mass = 82_kg
    material = [ skin, muscle, bone ]
    emits = [ heat, sweat ]
    metabolism {
        rate = 1300_kcal_day
        byproducts = [ CO2, H2O ]
    }
}
```

---

## 🧠 Core Concepts

### 🧩 Primitives
```
particle     // fundamental unit with defined properties
material     // bonded particles with mass/behavior
field        // space filled with forces or energy
```

### 🌐 Topology
```
thing        // collection of particles/materials with physical extent
space        // bounded or open volume in which things exist
environment  // space + fields + boundary conditions
```

### 🔁 Causality
```
interaction  // collision, reaction, radiation
law          // named physical law or equation
trigger      // condition that causes change in state
```

---

## 🧭 Why ARIA?

Other DSLs describe how things *look*.  
ARIA describes how things *are*.

- Create fully programmable **materials** and **scenes**
- Reference modular **laws** from `0_ordinem`
- Control **energy**, **mass**, and **time** directly
- Compose testable simulations in `4_lab/`

---

## 🚀 Roadmap

- [x] Grammar specification
- [ ] Lexer + Parser (`aria/parser.rs`)
- [ ] Interpreter and evaluator
- [ ] Compiler → Rust or runtime structure
- [ ] Live testing support inside `lab/`
- [ ] AI-assisted suggestion and validation

---

## 🧰 Example Scenarios

- Define a **new material** with 4 bonded particles and a unique absorption curve
- Simulate a **solar panel** by combining photon interaction, heat transfer, and electric potential
- Create a **fictional gas** that ignites under time-varying gravitational compression
- Design an **environmental envelope** with real-time radiation and entropy exchange

---

## 📂 Folder Structure

```
6_lang/aria/
├── grammar/
│   ├── syntax.rs
│   ├── types.rs
│   └── rules.rs
├── parser.rs
├── interpreter.rs
├── compiler.rs
├── bridge.rs       # Hooks into simulation runtime
├── examples/
│   └── cube_sim.aria
└── mod.rs
```

---

## 🧑‍🚀 Get Involved

We are actively designing and building ARIA. If you:

- Have experience building DSLs
- Love physics and expressive code
- Want to help simulate reality better than any engine in history…

Then you’re already part of this.  
ARIA is not just a language — it’s an interface to existence.

Let there be Light. Let it be written in ARIA.
