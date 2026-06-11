# Courses Platform

Web platform for hands-on AWS workshops: an axum server serves each course's lab
guides, and a scenario console, while staying course-agnostic. Course content lives
under `content/`, one subdirectory per course.

## Language

The domain speaks Spanish to users, and English in code: identifiers, types,
comments, and developer docs are English; all user-facing strings, and course
content, are Spanish. The glossary maps both.

**Course** (es: *taller*):
One deliverable workshop — a slug, a title, and its guide content.
_Avoid_: workshop (in code), training, class

**CourseSlug**:
The lowercase-kebab identifier of a Course (e.g. `aws-devops`); the only constructor is `parse`.
_Avoid_: id, name, key

**GuideSection** (es: *sección de la guía*):
One titled block of a Course's guide, carrying trusted, authored HTML.
_Avoid_: chapter, page, module

**Content root** (`content/`):
The directory holding each Course's authored content, keyed by CourseSlug.
_Avoid_: assets, static

**Scenario** (es: *escenario*) — deferred:
A user-triggered action that provokes observable infrastructure behavior (CPU burst, error-log burst, custom metric). Not in the backbone.
_Avoid_: demo, simulation, button

## Relationships

- A **Course** is identified by exactly one **CourseSlug**
- A **Course** has one or more **GuideSections**
- The **Content root** holds one subdirectory per **Course**, named by its **CourseSlug**
- A **Course** will own its **Scenarios** (future)

## Example dialogue

> **Dev:** "Is the *taller* a different thing from a **Course**?"
> **Domain expert:** "No — *taller* is the Spanish, user-facing name; in code it is always **Course**. One Course is one deliverable workshop."
> **Dev:** "And a **GuideSection** — is that a week of the workshop?"
> **Domain expert:** "Not necessarily. It is one titled block of the guide; how sections map to weeks, sessions, or labs is a content decision, not a type decision — for now."

## Flagged ambiguities

- Page language: `lang="es"` is hard-coded in `render_guide_page` — resolved: acceptable while every Course is Spanish; it becomes a **Course** field the day a non-Spanish Course exists.
