# CONTRIBUTING to AGENTS.md — Human Governance Guide

> [!CAUTION]
> **Warning:** This document is NOT for AI agents. It is a governance guide for human contributors who edit `AGENTS.md`. AI must never read or process this file.

---

## 📋 Table of Contents

1. [Why This File Exists](#-why-this-file-exists)
2. [Research Findings](#-research-findings)
   - [1. Instruction File Structure & Impact (Instructions-as-Code)](#1-instruction-file-structure--impact-instructions-as-code)
   - [2. Rule Polarity & Context Priming (Guardrails vs. Guidance)](#2-rule-polarity--context-priming-guardrails-vs-guidance)
   - [3. Repository Context Overhead & Minimalist Design (AGENTS.md Evaluation)](#3-repository-context-overhead--minimalist-design-agentsmd-evaluation)
   - [4. Closed-Loop Review Feedback & Behavioral Rule Accumulation (Self-Improving Agents)](#4-closed-loop-review-feedback--behavioral-rule-accumulation-self-improving-agents)
3. [Contributor Conclusions](#-contributor-conclusions)
   - [Sub-Agent Usage and Setup Guidelines](#sub-agent-usage-and-setup-guidelines)
   - [Performance and Flow Configuration](#performance-and-flow-configuration)
   - [Documentation File Method](#documentation-file-method)
4. [Note to Maintainers: AI Feedback Loop](#-note-to-maintainers-ai-feedback-loop)

---

## 🎯 Why This File Exists

`AGENTS.md` is an **LLM system prompt**, not human documentation. It lives inside the model's context window on every turn, so every wasted token degrades performance and burns budget. This guide explains what belongs in `AGENTS.md`, what does not, and why.

Use `AGENTS.md` to document solutions for recurring issues encountered during LLM workflows:

* 🎯 **Document Explicit Rules:** If the model repeatedly gets stuck or requires manual correction on the same mistake, specify its exact required behavior.
* 🌐 **Global Scope Only:** Ensure rules apply globally across the entire project rather than targeting isolated, one-off scenarios.

---

## 🔬 Research Findings

> [!IMPORTANT]
> Before making changes to the [AGENTS.md](/AGENTS.md) file, you must thoroughly read or analyze the papers listed below with the assistance of AI to fully understand them. To contribute, feel free to add any new research you find useful to this list.

### 1. Instruction File Structure & Impact (Instructions-as-Code)

Adding instruction files (`AGENTS.md`, `.cursorrules`, `copilot-instructions.md`) only improves agent performance when structured as modular code with fine-grained rules.

> 📄 **Source:** [arXiv:2606.13449](https://arxiv.org/html/2606.13449v1): Empirical study of 15,549 AI-generated PRs across 148 open-source GitHub projects (AIDev dataset).

#### Key Findings & Metrics
* **Success Split:** 27.70% of projects (41) saw $\ge 20\%$ higher PR merge rates, 26.35% (39) saw $\ge 20\%$ *decrease*, and 45.95% (68) had no significant change.
* **Structure Matters:** Successful instruction files were longer (median of **976 words** vs. **569 words** in failing projects) and heavily sectioned with H3 headers (median **9 `###` headers** vs. **1 `###` header** in failing projects).
* **Code Quality & Friction:** Well-structured instruction files reduced code churn by **-18.5%** and review comments to **0.6x**, whereas poor files increased code churn by **+34.2%** and review comments to **1.8x**.

> [!TIP]
> **Rule:** Write instruction files as modular, fine-grained **`###` subsections** (*Instructions-as-Code*) rather than loose, informal text.

---

### 2. Rule Polarity & Context Priming (Guardrails vs. Guidance)

Prohibitive guardrails ("DO NOT") consistently improve agent task completion, whereas prescriptive style guidance ("DO") is often neutral or individually harmful.

> 📄 **Source:** [arXiv:2604.11088](https://arxiv.org/html/2604.11088v2): Controlled study analyzing 679 rule files (25,532 rules) across 5,000+ Claude Code agent runs on SWE-bench Verified.

#### Key Findings & Metrics
* **Guardrails Beat Guidance:** Prohibitive negative constraints (e.g., *"do not refactor unrelated code"*) consistently boosted task completion rates by **+13.8 percentage points (+13.8pp)**, whereas positive directives (e.g., *"follow code style"*, *"run tests before committing"*) were neutral or harmful (-1.2pp to +0.4pp).
* **Context Priming Mechanism:** Performance gains were largely content-independent; randomized, shuffled, or off-domain rule sets matched expert-curated rule sets (yielding the same **+13.8pp** gain), demonstrating that rule files function primarily as a context priming signal that activates structured problem-solving pathways.
* **Ensemble Resilience:** Accumulating 0 to 50 rules maintained stable pass rates without degrading performance.
* **Aviation Checklist Analogy:** Like flight checklists, rule files work best when preventing specific human/LLM error modes rather than teaching baseline procedural knowledge.

> [!TIP]
> **Rule:** Prioritize explicit negative constraints and prohibitive guardrails over positive style advice to prevent agent missteps and activate structured reasoning.

---

### 3. Repository Context Overhead & Minimalist Design (AGENTS.md Evaluation)

Comprehensive repository context files often degrade agent task performance and inflate inference costs unless restricted to minimal, essential constraints.

> 📄 **Source:** [arXiv:2602.11988](https://arxiv.org/html/2602.11988v1): Empirical study evaluating LLM-generated and human-written context files across 438 coding tasks in SWE-bench Lite and AGENTbench.

#### Key Findings & Metrics
* **Task Success Degradation:** Providing repository-level context files generally reduced overall task completion rates compared to providing no context at all; LLM-generated context files decreased success rates by **-3%**, while human-written files yielded only a marginal **+4%** gain.
* **Inference Cost Surge:** Including repository context files increased total inference costs by **>20%** across all evaluated agent frameworks and LLMs.
* **Increased Exploration Friction:** Agents actively obeyed context guidelines—increasing tool usage by **1.6x to 2.5x** and executing more tests—yet high-level repository overviews failed to help agents locate relevant files faster, causing agents to exhaust step/token budgets.
* **Ablation Results:** Stronger LLMs did not generate more effective context files, and prompt engineering variations failed to eliminate performance drops.

> [!TIP]
> **Rule:** Omit verbose codebase overviews and comprehensive guidelines from repository context files; restrict instructions strictly to minimal, non-negotiable execution constraints.

---

### 4. Closed-Loop Review Feedback & Behavioral Rule Accumulation (Self-Improving Agents)

Converting accepted human PR review comments into persistent, version-controlled behavioral rules eliminates repetitive error classes across sessions without model fine-tuning.

> 📄 **Source:** [arXiv:2607.13091](https://arxiv.org/html/2607.13091v1): Deployment study of a closed-loop framework across 35+ microservices, evaluating 11 logged work sessions and 36 PR reviews.

#### Key Findings & Metrics
* **Zero Error Recurrence:** Codifying human PR review feedback into explicit behavioral rules achieved a **0% recurrence rate** (0 repeated errors across 74 post-rule session exposures) across all codified error classes.
* **Review Effort Elevation:** Human review effort shifted away from low-level mechanical correctness and code formatting (**14%** of total review comments) toward high-level architecture, design, and performance (**66%** of total review comments).
* **Logarithmic Rule Saturation:** Rule set accumulation followed a logarithmic curve, stabilizing at **18 behavioral rules**, **15+ language-specific standards**, and a **15-item self-review checklist** (~6,250 tokens, consuming **<5%** of a 128K context window).
* **Cross-Domain & Tool Transfer:** **60%** of documented knowledge transfers crossed repository, task, or interface boundaries (e.g., migrating rules seamlessly from IDE to Terminal agents) without requiring model weight updates or RLHF.

> [!TIP]
> **Rule:** Systematically transform accepted human code review comments into version-controlled instruction rules and pre-submission self-review checklists to permanently block recurring error classes.

---

## 💡 Contributor Conclusions

> [!NOTE]
> **Note:** If you find any section to be incorrect or inaccurate, please add your corrections along with supporting evidence and references.

### Sub-Agent Usage and Setup Guidelines

A **sub-agent** is an isolated worker process spawned by a primary (orchestrator) agent to execute a specific, self-contained task. By running within its own fresh context window and passing only the latest chat message rather than the entire conversation history, a sub-agent prevents context window saturation, lowers inference costs, and maintains high execution accuracy across complex workflows.

#### When to Use vs. When Not to Use Sub-Agents

##### ✅ When to Use:
* **Context Window Saturation:** Used when the context window becomes bloated to prevent the LLM from losing focus due to excessive context.
* **Cost Optimization:** To keep costs low, assign a frontier model to the main agent (orchestrator) while utilizing cheaper models for certain sub-agents (excluding planning agents).
* **Context Isolation:** Applied to offload the main agent's context window, prevent context poisoning, and achieve more accurate results. *(Note: The sub-agent should inherit minimal context from the main agent. Only include critical execution constraints and rules that would break the code if violated.)*
* **Parallel Tasks:** When parallel execution is required (tasks must be mutually independent to avoid conflicts; the main agent should ideally determine this suitability).
* **Separation of Roles and Responsibilities:** When a single model is instructed in `AGENTS.md` to simultaneously write code, run tests, and act as a security auditor, it bloats the context window and significantly increases the likelihood of errors.

##### ❌ When Not to Use:
* For sequential, interdependent tasks.
* For simple / single-session tasks.

#### Sub-Agent Setup Guidelines

* **Fresh Context Isolation:** When spawning a sub-agent, avoid passing the entire conversation history from the parent agent. Forward only the data directly relevant to the sub-agent's assigned task.

---

### Performance and Flow Configuration

Avoid exhausting system limits by cluttering the framework with complex sub-agent and skill files for negligible (~1%) performance gains.

#### Key Principles for Improving Accuracy

* **Establish Boundaries:** Explicitly instruct the model on what **not** to do.
* **Provide Concrete Commands:** Demonstrate **how** to execute tasks using concrete code snippets and exact commands rather than abstract descriptions.
  > *(e.g., Instead of adding "run tests before committing" to the documentation, include the exact test command that must be executed.)*

LLMs already understand general task workflows. Unless specifying project-specific details that the model cannot infer, avoid overloading it with excessive instructions; otherwise, performance degradation is inevitable.

> Instead of forcing information onto the LLM, help it find the information.

---

### Documentation File Method

The use of documentation files such as `architecture.md` or `doc/001_feature.md` was considered, but abandoned due to development budget constraints.

* These documents must be rewritten with every modified line of code. Failing to update them when code is written or edited creates confusion for humans while causing context poisoning in LLMs, leading the model to hallucinate.

#### Contributor's Solution

We can solve this issue by creating a sub-agent named `codebase-explorer` and instructing it on how to search the codebase. This sub-agent can explain the relevant code details to the main agent (orchestrator) based on the requested information. This approach resolves the problem while preventing the main agent's context window from unnecessarily filling up. **@voidvore**

---

## 🔄 Note to Maintainers: AI Feedback Loop

> [!IMPORTANT]
> **Attention Maintainers:** When an LLM-generated error is identified during a PR review, evaluate the following:  
> *"Is this a one-off issue specific to this task, or a general error class that the AI might repeat in the future?"*
> 
> * **If it is a one-off issue:** Simply request the code fix via a PR comment.
> * **If it is a general error class (Closed Loop):** In addition to the code fix, ask the PR author (or update it yourself) to update the [AGENTS.md](/AGENTS.md) file within the same PR.
> 
> **Sections to Update in AGENTS.md:**
> 1. **Code Standards / Behavioral Rules:** Add the constraint and rationale as a clear rule.
> 2. **Anti-Patterns:** Provide an example of the forbidden code pattern and, if necessary, its correct alternative.