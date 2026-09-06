---
author: Ray
title: "Glossary"
description: プロジェクト固有の用語やアーキテクチャの定義
---

# プロジェクト用語集 (Glossary)

このドキュメントは、AI エージェントがコードベースを理解し、実装する際に従うべきドメイン用語やアーキテクチャ上の定義をまとめたものです。
AI エージェントは作業を開始する前に必ずこのドキュメントを読み込み、定義に沿って命名や設計を行ってください。

## 1. ドメイン用語定義

出典: `docs/domain/EVIDENCE_MODEL.md`, `docs/domain/PRODUCTION_SYSTEM_MODEL.md`, `docs/domain/TRANSFORMATION_STAGE_MODEL.md`, `docs/domain/RANKING_MODEL.md`, `docs/scoring/SCORING_SPEC.md`。

| 用語 (英語) | 用語 (日本語) | 定義 / 制約 |
| :--- | :--- | :--- |
| **EvidenceRecord** | 証拠レコード | 判断チェーンに入るすべての事実が遡れる、来源付きの最小単位。事実 ID・企業・観測時刻・有効日・来源種別・URI・主張・極性・信頼度・鮮度・抽出バージョン・内容ハッシュを保持する。 |
| **Supporting / Counter / Missing Evidence** | 支持証拠・反証・欠落証拠 | 候補企業ごとに三種類を同時に維持する。反証審査を経ていない候補は `Top5` に入れない。権威ある事実がない場合は経験で埋めず `UNKNOWN` / `UNAVAILABLE` とする。 |
| **Evidence Candidate** | 証拠候補 | 外部情報をルールベース抽出しただけの未検証状態。Rust側の検証・正規化を経て初めて Stage/Score の入力になれる。外部テキストはデータであり指示ではない。 |
| **ProductionSystem / ProductionUnit / ProductionWorkflow** | 生産システム／生産単位／生産ワークフロー | 企業が中核価値を生み出す全体の仕組み・可識別な産出を担う単位・意図から納品/検証/例外処理までの経路。 |
| **ControlPoint / VerificationPoint / DecisionPoint / ExceptionPath** | 制御点／検証点／意思決定点／例外経路 | 生産ワークフロー内で、人間または仕組みが統制を保持する点／産出が検査・却下される点／目標や責任が発生する点／自動化が処理できない場合の人手介入経路。 |
| **TransformationStage** (`TOOL`〜`REFERENCE_MODEL`) | 転型ステージ（6段階） | `TOOL → SUBSTITUTION → WORKFLOW → PRODUCTION_SYSTEM → PRODUCTIVITY_BREAKOUT → REFERENCE_MODEL`。単発のニュースや宣伝ではステージは上がらない。上位ステージほど複数証拠カテゴリ・独立来源・持続性が必要（詳細: `docs/domain/TRANSFORMATION_STAGE_MODEL.md`）。 |
| **ReferenceModel** | リファレンスモデル | `OrganizationRewrite` / `ProductionSystemRewrite` / `SustainedOutcome` / `IndustryDiffusion` の4種の来源付き主張が揃って初めて `Candidate → Confirmed` に進む、業界模範に対する最も厳格な判定レイヤー。 |
| **SupplierAttribution / IndependentCustomerDisclosure** | サプライヤー起因の言及／独立した採用者開示 | `IndustryDiffusion` の来源は役割で区別する。サプライヤーが顧客事例を語るだけでは独立検証にならず、採用者自身の開示（IR・経営情報）のみが独立拡散証拠として数えられる。 |
| **Transformation Score** | 転型スコア | 同一 Stage 内でのみ有効な補助比較指標であり、Stage の代替にはならない。`Evidence Confidence` / `Counter Evidence Risk` / `Evidence Freshness` は単一スコアに隠さず個別に保持する。 |
| **Ranking Order** | ランキング順序 | 比較優先順位は `Stage → Evidence Confidence → Transformation Score → Counter Evidence Risk → Evidence Freshness` の固定順。高信頼の Stage 4 は高スコアの Stage 1 より常に優先される。 |
| **Top5 / Rising / Watch / Dropped** | Top5・上昇・監視・除外 | 研究優先度を表す4つの view。投資助言や売買シグナルではない（`NORTH_STAR.md`, ADR-009）。`Dropped` は誤判定を訂正するための正式な結果であり失敗ではない。 |
| **AI_THEATER_RISK** | AIシアターリスク | AI への言及だけが増え、実際のワークフロー・責任・検証構造が変化していない場合に付与され、Top5 判定を降格させるカウンターシグナル（`docs/scoring/SCORING_SPEC.md`）。 |

## 2. アーキテクチャ境界

`src/features/<context>/` 配下は DDD 風の境界づけられたコンテキストごとに、以下5層を必ず持つ（`tests/architecture/module_boundaries.rs` が層の存在を、`tests/architecture/dependency_rules.rs` が依存方向をそれぞれ機械的に強制する）。

- **domain**: 純粋なビジネスルールとエンティティ/値オブジェクトのみを含む。`infrastructure` / `provider` / 外部 crate（HTTP・JSON 変換等）への依存を禁止する。
- **application**: ドメインを組み合わせてユースケースを実行する層。外部I/Oは infrastructure に委譲する。
- **infrastructure**: SEC EDGAR/XBRL 取得（`sec.rs`）、Telegram配信、ファイルアーカイブなど外部システムへのアクセスを担当する。ドメインロジックをここに書かない。
- **interface**: CLI/レンダリング等、外部との入出力境界。バリデーション以外のビジネスロジックはドメイン/アプリケーション層に委譲する。
- **acl** (Anti-Corruption Layer): 他コンテキストの型が自コンテキストのドメインに直接漏れ込むのを防ぐ変換層。

現状 `weekly_radar` と `validation` のみ全層が実装済みで、他10コンテキスト（diffusion, evidence, ingestion, organization, production_system, productivity, ranking, reporting, transformation, universe）は値オブジェクト中心のスキャフォールド段階にある。新規実装時はこの区別を踏まえ、スキャフォールドへ実装を追加する前に対象コンテキストの位置づけを確認すること。

## 3. AI Cockpit Governance Terms

AI Cockpit を採用しているリポジトリでは、次の用語を分散配布する glossary に含めてください。

- **Preflight Review**: 実装前に Contract の証拠から派生する readiness の助言ビュー。
- **Preflight Pause Rule**: `needs_human_confirmation`、`human_decision_recorded`、`not_ready` のとき、エージェントはユーザーへレビューを報告し、新しく再計算された `ready` の証拠が得られるまで実装または finish へ進まない。
- **Planned Scenario Verification**: 実装後にしか検証できない必須シナリオについて、Contract に期待結果と具体的な `verificationPlan` を記録して実装準備を示す状態。検証完了の証拠ではなく、Summary と finish は実行済み証拠が揃うまで fail closed を維持する。
- **Evidence over Self-Declaration**: readiness を AI の自己申告ではなく、既存証拠から派生させる原則。
