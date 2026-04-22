---
source: mixed
---

# Problem Taxonomy — Analytics and Machine Learning

A cheat-sheet for identifying what kind of problem you have and which family of methods to reach for.

## Basic Data Analytics

| Problem family | Typical goal | Common examples | Usual methods |
|---|---|---|---|
| Descriptive statistics | Summarize what happened | Mean, median, variance, distributions | Aggregation, summary tables, visualization |
| Data cleaning | Fix missing, messy, or inconsistent data | Missing values, duplicates, outliers | Imputation, filtering, validation rules |
| Querying / reporting | Retrieve and summarize slices of data | Sales by month, churn by region | SQL, OLAP, dashboards |
| Exploratory analysis | Find patterns and anomalies | Correlations, trends, segments | Scatter plots, clustering, pivot tables |
| Dimensionality reduction | Compress many variables into fewer ones | PCA, factor analysis | Linear algebra methods |
| Forecasting | Predict future numeric values | Demand, revenue, traffic | Time-series models, regression |
| Segmentation | Group similar records | Customer segments, product groups | Clustering, rule-based grouping |
| Anomaly detection | Find unusual records or events | Fraud, sensor faults, spikes | Statistical thresholds, isolation methods |

## Machine Learning

| Problem family | Typical goal | Common examples | Usual methods |
|---|---|---|---|
| Classification | Predict a category | Spam/not spam, disease/no disease | Logistic regression, trees, SVM, neural nets |
| Regression | Predict a number | House price, demand, risk score | Linear regression, random forest, gradient boosting |
| Ranking / recommendation | Order items by relevance | Search results, product recommendations | Learning-to-rank, collaborative filtering |
| Clustering | Group unlabeled data | Customer groups, topic discovery | K-means, hierarchical clustering, DBSCAN |
| Dimensionality reduction | Reduce feature count | PCA, embeddings, autoencoders | PCA, t-SNE, UMAP, autoencoders |
| Time-series prediction | Predict over time | Forecasting, anomaly detection | ARIMA, state-space models, RNNs, transformers |
| Detection / segmentation | Find objects or regions | Image detection, medical segmentation | CNNs, U-Nets, vision transformers |
| Reinforcement learning | Learn actions from reward | Games, robotics, control | Q-learning, policy gradients, actor-critic |

## Quick Decision by Output Type

| If the output is... | Reach for... |
|---|---|
| A number | Regression or forecasting |
| A category | Classification |
| A grouping | Clustering or segmentation |
| A ranking | Recommendation or learning-to-rank |
| A future action under rewards | Reinforcement learning |

## Analytics vs Machine Learning

- **Analytics** asks: what happened, why did it happen, and what might happen next.
- **Machine learning** asks: can we learn a predictive or decision-making rule from data.

Analytics often feeds ML — cleaning, exploration, and feature engineering come first. The top-level ML split is supervised learning, unsupervised learning, and reinforcement learning.

## Relation to Optimization

Unlike optimization problems (knapsack, scheduling, assignment), analytics and ML problems are primarily about **learning from data** rather than choosing the best solution under explicit constraints. Some ML methods use optimization internally, but the modeling goal is different.

See `kb/Algorithms/` for individual algorithm pages covering both families.
