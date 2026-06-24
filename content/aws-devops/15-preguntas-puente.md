+++
title = "Preguntas puente"
+++

Estas preguntas cierran la sesión presencial y abren la remota. En la sesión de hoy se operó el workload
—red, escalado, fallas— y se vio qué es un pipeline. Conviene reflexionarlas antes de la sesión remota,
donde se construye el pipeline, se conecta a las notificaciones, y se abre la
observabilidad.

:::slide
## Preguntas puente

1. ¿Qué etapas necesitaría el pipeline de la aplicación, y en qué orden?
2. ¿Dónde agrega valor una aprobación manual, y dónde solo estorba?
3. Cuando el build termina, ¿quién debería enterarse y por qué medio?
:::

---

## Pregunta 1

Para automatizar el flujo que hoy se hace a mano, ¿qué etapas necesitaría un pipeline de la
aplicación, y en qué orden?

::: solucion
Tres etapas, en este orden:

1. **Source** — obtener el código desde CodeCommit. Es lo que dispara el pipeline: un
   commit nuevo en la rama vigilada.
2. **Build** — ejecutar el proyecto de CodeBuild, que construye la imagen Docker y la
   publica en ECR (el `buildspec.yml` correspondiente).
3. **Deploy** — actualizar el servicio de ECS para que tome la nueva imagen.

El orden no es arbitrario: cada etapa consume el artefacto de la anterior. No se puede
construir sin el código, ni desplegar sin la imagen. Es la misma secuencia que ejecuta
hoy manualmente, ahora descrita una vez.
:::

---

## Pregunta 2

¿Dónde, en ese flujo, agrega valor una aprobación manual? ¿Y dónde solo lo haría más
lento sin aportar nada?

::: solucion
Una **aprobación manual** agrega valor **antes de la etapa de Deploy**: es el punto donde
alguien confirma que la imagen recién construida puede salir a producción. Da una pausa
deliberada entre "se construyó" y "se desplegó", útil cuando el despliegue es delicado o
requiere coordinación.

Ponerla **antes del Build** solo molestaría: construir es barato, reproducible, y no
afecta a nadie —no hay nada que aprobar todavía. La regla práctica: las aprobaciones
manuales van delante de las acciones **irreversibles o visibles para el usuario**, no
delante de los pasos internos y repetibles.
:::

---

## Pregunta 3

Cuando el build termina —en éxito o en error— ¿quién debería enterarse, y por qué medio?

::: solucion
Debe enterarse **el equipo**, por el canal donde ya conversa —en esta organización,
**Microsoft Teams**— sin tener que mirar la consola. Un build exitoso confirma que el
cambio avanzó; uno fallido necesita atención rápida, y cuanto antes se detecte, antes se
corrige.

El mecanismo que lo hace posible es el que adelantamos en la Semana 1: el evento del
build lo capturan las reglas de notificación, lo publican en SNS, y AWS Chatbot lo
entrega a Teams. En el laboratorio lo veremos como un *toast* en la guía del instructor,
que es un espejo de ese mismo flujo. Lo construimos en la próxima sesión.
:::
