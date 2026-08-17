---
author: Ray
title: "Glossary"
description: プロジェクト固有の用語やアーキテクチャの定義
---

# プロジェクト用語集 (Glossary)

このドキュメントは、AI エージェントがコードベースを理解し、実装する際に従うべきドメイン用語やアーキテクチャ上の定義をまとめたものです。
AI エージェントは作業を開始する前に必ずこのドキュメントを読み込み、定義に沿って命名や設計を行ってください。

## 1. ドメイン用語定義

*以下はサンプル行です。プロジェクト固有の用語に置き換えてください。*

| 用語 (英語) | 用語 (日本語) | 定義 / 制約 |
| :--- | :--- | :--- |
| **[Core Concept A]** | [日本語名] | [定義と命名規則。例：`EntityNameService` の形式で命名すること。] |
| **[Core Concept B]** | [日本語名] | [定義と設計上の制約。例：このエンティティはドメイン層にのみ存在すること。] |

## 2. アーキテクチャ境界

以下はアーキテクチャ境界の例です。プロジェクトの設計に合わせて書き換えてください。

- **Infrastructure Layer**: 外部データベースやキューへのアクセスを担当するレイヤー。ドメインロジックをここに書かないこと。
- **Domain Layer**: 純粋なビジネスルールを含むレイヤー。フレームワークやデータベースライブラリへの直接の依存を禁止する。
- **Presentation Layer**: HTTP / API エントリーポイント。バリデーション以外のビジネスロジックはドメインサービスに委譲すること。

## 3. AI Cockpit Governance Terms

AI Cockpit を採用しているリポジトリでは、次の用語を分散配布する glossary に含めてください。

- **Preflight Review**: 実装前に Contract の証拠から派生する readiness の助言ビュー。
- **Preflight Pause Rule**: `needs_human_confirmation`、`human_decision_recorded`、`not_ready` のとき、エージェントはユーザーへレビューを報告し、新しく再計算された `ready` の証拠が得られるまで実装または finish へ進まない。
- **Planned Scenario Verification**: 実装後にしか検証できない必須シナリオについて、Contract に期待結果と具体的な `verificationPlan` を記録して実装準備を示す状態。検証完了の証拠ではなく、Summary と finish は実行済み証拠が揃うまで fail closed を維持する。
- **Evidence over Self-Declaration**: readiness を AI の自己申告ではなく、既存証拠から派生させる原則。
