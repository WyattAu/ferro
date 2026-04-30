terraform {
  required_version = ">= 1.0"
  required_providers {
    k3s = {
      source  = "k3s-io/k3s"
      version = "~> 1.0"
    }
  }
}

provider "k3s" {
  kubeconfig_raw = var.kubeconfig_raw
}

resource "k3s_namespace_v1" "ferro" {
  metadata {
    name = var.namespace
    labels = {
      "app.kubernetes.io/name"    = "ferro"
      "app.kubernetes.io/part-of" = "ferro"
    }
  }
}

resource "k3s_manifest" "ferro" {
  content = file("${path.module}/../k3s/ferro.yaml")

  depends_on = [k3s_namespace_v1.ferro]
}
