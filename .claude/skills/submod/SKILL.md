```markdown
# submod Development Patterns

> Auto-generated skill from repository analysis

## Overview
This skill teaches you the core development patterns and workflows used in the `submod` Rust repository. `submod` manages Git submodules, focusing on toggling and tracking their "active" status. You'll learn the project's coding conventions, how to update submodule activation logic, and how to maintain and test your changes effectively.

## Coding Conventions

### File Naming
- Use **snake_case** for file and module names.
  - Example: `git_manager.rs`, `git2_ops.rs`

### Import Style
- Use **relative imports** within the crate.
  - Example:
    ```rust
    use crate::git_ops::git2_ops;
    ```

### Export Style
- Use **named exports** for functions and structs.
  - Example:
    ```rust
    pub fn update_active_status(...) { ... }
    pub struct Submodule { ... }
    ```

### Commit Messages
- Follow **conventional commit** style.
- Use the `feat` prefix for new features.
  - Example: `feat: add support for toggling submodule active status`

## Workflows

### Update Submodule Active Status
**Trigger:** When you need to change how submodule activation is tracked or persisted, especially regarding the `active` field in `.gitmodules`.
**Command:** `/update-submodule-active`

1. **Modify submodule management logic**  
   Update the logic in `src/git_manager.rs` to reflect changes in how the `active` status is handled.
   ```rust
   // Example: Toggle active status
   pub fn set_active(&mut self, active: bool) {
       self.active = active;
   }
   ```
2. **Update serialization and writing logic**  
   Ensure that both `src/git_ops/git2_ops.rs` and `src/git_ops/gix_ops.rs` correctly serialize and persist the `active` field.
   ```rust
   // Example: Serialize 'active' field
   let submodule_data = format!("active = {}", submodule.active);
   ```
3. **Update or add tests**  
   Add or update tests in `tests/git_ops_tests.rs` and `tests/integration_tests.rs` to cover the new or changed behavior.
   ```rust
   #[test]
   fn test_active_status_toggle() {
       // ...test logic...
   }
   ```
4. **Update dependencies (optional)**  
   If your changes require new dependencies, update `Cargo.lock`.

## Testing Patterns

- **Test Framework:** Not explicitly detected; standard Rust testing conventions are used.
- **Test File Pattern:** Test files are named with `*.test.*` (e.g., `git_ops_tests.rs`).
- **Example Test:**
  ```rust
  #[test]
  fn test_submodule_activation() {
      // Arrange
      let mut submodule = Submodule::new("example");
      // Act
      submodule.set_active(true);
      // Assert
      assert!(submodule.active);
  }
  ```

## Commands

| Command                   | Purpose                                                      |
|---------------------------|--------------------------------------------------------------|
| /update-submodule-active  | Update logic and persistence for submodule active status      |
```
