variable "admin_password" {
  description = "Admin password for Ferro"
  type        = string
  sensitive   = true
}

variable "admin_user" {
  description = "Admin username for Ferro"
  type        = string
  default     = "admin"
}

variable "replica_count" {
  description = "Number of Ferro replicas"
  type        = number
  default     = 1
}

variable "image_repository" {
  description = "Container image repository"
  type        = string
  default     = "ferro"
}

variable "image_tag" {
  description = "Container image tag"
  type        = string
  default     = "latest"
}

variable "persistence_enabled" {
  description = "Enable persistent storage"
  type        = bool
  default     = true
}

variable "persistence_size" {
  description = "Size of the persistent volume"
  type        = string
  default     = "10Gi"
}

variable "ingress_enabled" {
  description = "Enable Ingress resource"
  type        = bool
  default     = true
}

variable "ingress_class_name" {
  description = "Ingress class name"
  type        = string
  default     = "nginx"
}

variable "network_policy_enabled" {
  description = "Enable network policies"
  type        = bool
  default     = true
}
