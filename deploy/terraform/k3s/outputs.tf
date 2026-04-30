output "namespace" {
  description = "Kubernetes namespace for Ferro"
  value       = var.namespace
}

output "manifest_applied" {
  description = "Whether the K3s manifest was applied"
  value       = k3s_manifest.ferro.id != ""
}
