terraform {
  required_version = ">= 1.5"

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
  }

  backend "gcs" {
    prefix = "prod/converge-runtime"
  }
}

provider "google" {
  project = var.project_id
  region  = var.region
}

variable "project_id" {
  type = string
}

variable "region" {
  type    = string
  default = "europe-west1"
}

variable "service_name" {
  type    = string
  default = "converge-runtime"
}

variable "repository_id" {
  type    = string
  default = "converge"
}

variable "runtime_image_name" {
  type    = string
  default = "converge-runtime"
}

variable "runtime_image_tag" {
  type    = string
  default = "latest"
}

variable "firebase_project_id" {
  type    = string
  default = ""
}

variable "allow_unauthenticated" {
  type    = bool
  default = true
}

variable "plain_env_vars" {
  type    = map(string)
  default = {}
}

variable "secret_env_vars" {
  type = map(object({
    secret_id = string
    version   = string
  }))
  default = {
    OPENAI_API_KEY = {
      secret_id = "converge-openai-api-key"
      version   = "latest"
    }
    ANTHROPIC_API_KEY = {
      secret_id = "converge-anthropic-api-key"
      version   = "latest"
    }
    GEMINI_API_KEY = {
      secret_id = "converge-gemini-api-key"
      version   = "latest"
    }
    BRAVE_API_KEY = {
      secret_id = "converge-brave-api-key"
      version   = "latest"
    }
  }
}

locals {
  firebase_project_id = var.firebase_project_id != "" ? var.firebase_project_id : var.project_id
  runtime_image_url   = "${module.artifact_registry.repository_url}/${var.runtime_image_name}:${var.runtime_image_tag}"

  runtime_env = merge({
    PORT                      = "8080"
    RUST_LOG                  = "info"
    LOCAL_DEV                 = "false"
    DISABLE_AUTH              = "false"
    GCP_PROJECT_ID            = var.project_id
    GOOGLE_CLOUD_PROJECT      = var.project_id
    FIREBASE_PROJECT_ID       = local.firebase_project_id
    CONVERGE_RUNTIME_FEATURES = "gcp,auth,firebase"
  }, var.plain_env_vars)
}

resource "google_project_service" "apis" {
  for_each = toset([
    "run.googleapis.com",
    "artifactregistry.googleapis.com",
    "secretmanager.googleapis.com",
    "cloudbuild.googleapis.com",
    "firestore.googleapis.com",
  ])

  project            = var.project_id
  service            = each.value
  disable_on_destroy = false
}

module "artifact_registry" {
  source        = "../../../modules/artifact-registry"
  project_id    = var.project_id
  region        = var.region
  repository_id = var.repository_id

  depends_on = [google_project_service.apis]
}

module "runtime" {
  source       = "../../../modules/cloud-run-service"
  project_id   = var.project_id
  region       = var.region
  service_name = var.service_name
  image        = local.runtime_image_url

  port                  = 8080
  cpu                   = "1"
  memory                = "1Gi"
  min_instances         = 0
  max_instances         = 10
  allow_unauthenticated = var.allow_unauthenticated
  env_vars              = local.runtime_env
  secret_env_vars       = var.secret_env_vars

  depends_on = [google_project_service.apis]
}

resource "google_project_iam_member" "runtime_firestore" {
  project = var.project_id
  role    = "roles/datastore.user"
  member  = "serviceAccount:${module.runtime.service_account_email}"
}

output "registry_url" {
  value = module.artifact_registry.repository_url
}

output "runtime_url" {
  value = module.runtime.service_url
}

output "runtime_service_name" {
  value = module.runtime.service_name
}

output "runtime_service_account_email" {
  value = module.runtime.service_account_email
}
