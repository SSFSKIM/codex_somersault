export type FeedbackSurveyResponse =
  | 'bad'
  | 'ok'
  | 'good'
  | 'dismissed'
  | 'fine'
  | string
export type FeedbackSurveyType =
  | 'general'
  | 'memory'
  | 'post-compact'
  | 'skill-improvement'
  | string
