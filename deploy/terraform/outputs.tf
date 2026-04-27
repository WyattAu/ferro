output "namespace" {
  description = "Kubernetes namespace for Ferro"
  value       = kubernetes_namespace_v1.ferro.metadata[0].name
}

output "helm_release_name" {
  description = "Helm release name"
  value       = helm_release.ferro.name
}

output "helm_release_status" {
  description = "Helm release status"
  value       = helm_release.ferro.status
}
