terraform {
  required_version = ">= 1.0"
  required_providers {
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.23"
    }
    helm = {
      source  = "hashicorp/helm"
      version = "~> 2.11"
    }
  }
}

provider "kubernetes" {
  config_path = "~/.kube/config"
}

provider "helm" {
  kubernetes {
    config_path = "~/.kube/config"
  }
}

resource "kubernetes_namespace_v1" "ferro" {
  metadata {
    name = "ferro"
    labels = {
      "app.kubernetes.io/name"    = "ferro"
      "app.kubernetes.io/part-of" = "ferro"
    }
  }
}

resource "helm_release" "ferro" {
  name       = "ferro"
  chart      = "../helm/ferro"
  namespace  = kubernetes_namespace_v1.ferro.metadata[0].name
  wait       = true
  timeout    = 600

  set {
    name  = "replicaCount"
    value = var.replica_count
  }

  set {
    name  = "persistence.size"
    value = var.persistence_size
  }

  set {
    name  = "persistence.enabled"
    value = var.persistence_enabled
  }

  set {
    name  = "image.repository"
    value = var.image_repository
  }

  set {
    name  = "image.tag"
    value = var.image_tag
  }

  set {
    name  = "ingress.enabled"
    value = var.ingress_enabled
  }

  set {
    name  = "ingress.className"
    value = var.ingress_class_name
  }

  set {
    name  = "auth.adminUser"
    value = var.admin_user
  }

  set_sensitive {
    name  = "auth.adminPassword"
    value = var.admin_password
  }

  set {
    name  = "networkPolicy.enabled"
    value = var.network_policy_enabled
  }

  depends_on = [kubernetes_namespace_v1.ferro]
}
