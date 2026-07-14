import Lake
open Lake DSL

package «ferro-formal» where
  leanOptions := #[⟨`autoImplicit, false⟩]

@[default_target]
lean_lib «Ferro» where
  srcDir := "."
  roots := #[`Ferro]

lean_lib «Ferro.CircuitBreakerRefined» where
  srcDir := "."

lean_lib «Ferro.RateLimiterProperties» where
  srcDir := "."

lean_lib «Ferro.CacheConsistency» where
  srcDir := "."

lean_lib «Ferro.PathSafety» where
  srcDir := "."

lean_lib «Ferro.CRDTProperties» where
  srcDir := "."

lean_lib «Ferro.AuthTokenProperties» where
  srcDir := "."

lean_lib «Ferro.CacheInvalidation» where
  srcDir := "."
