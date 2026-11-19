/* `TestsInstall.swift`'s installer, which install.c's constructor calls as the
 * test bundle loads, and the flag it sets once it has run. */
void exact_tests_install(void);
extern int exact_tests_installed;

/* The archive's style grammars, which a session's boot links: a test reads
 * style values through the exports (`exact_gradient`, …) with no session,
 * so the bundle links them as it loads. The archive's (`host!`). */
void exact_link_grammars(void);
