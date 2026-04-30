variable "kubeconfig_raw" {
  description = "Raw K3s kubeconfig content"
  type        = string
  sensitive   = true
}

variable "namespace" {
  description = "Kubernetes namespace for Ferro"
  type        = string
  default     = "ferro"
}
