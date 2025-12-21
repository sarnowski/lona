We want to implement the next open task from our @docs/roadmap/index.md.

In order for you to understand enough background information to correctly plan the next step, you must read:

  * @docs/goals.md
  * @docs/lonala.md
  * @docs/minimal-rust.md

Pick the next open task in the roadmap and read the respective milestone document completely. Then check possible relevant existing code.

Consider the existing code base and state of implementation as well as the future target picture. Tell me if you see inconsistencies, or issues with the current plan and next steps. Also ask me questions if you need my input to better understand the goals and next steps and design decision. Our goal is to have CORRECT solutions and not hacks or quick fixes. We do NOT want to defer any code or feature. We want the best and most correct solution. You can always do some research in the Internet to also strengthen your understanding of the possible solution space and seek best practices from others.

After you got to a full understanding of the next task with all necessary information, create a plan for yourself. Do NOT keep any backwards compatibility. We strive for the optimal solution and expect refactoring to adopt the code base to new functions and patterns.

Then call both Gemini (CLI) and Codex (CLI) in parallel to create plans (don't give them your plan). Ask each to also do all the background research necessary (such as reading our docs and code as well as doing Internet research) and receive a plan back from each. Both should run with the best available model. Think about their plans and compare them with yours. Challenge your own plan with Gemini's and Codex's plans and think about the learnings that you can derive from them. Then review your own plan and check how to improve it in order to achieve optimal results by incorporating insights from all sources.

Finally I want you to think about how to most effectively implement the next task. If the task is too big for one context window, then split this up into multiple tasks. Present me your final plan for the next roadmap task.
