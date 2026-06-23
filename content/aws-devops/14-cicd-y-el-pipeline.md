+++
title = "CI/CD y el rol de un pipeline"
+++

## Lo que ya hace a mano

A lo largo del taller construyó, sin nombrarlo, un flujo de entrega completo. Cada vez
que cambia el código, hoy ejecuta a mano una secuencia: subir el commit a CodeCommit,
lanzar el build en CodeBuild, esperar la imagen en ECR, y actualizar el servicio de ECS
para que tome la nueva imagen.

Funciona, pero depende de que alguien recuerde los pasos, los ejecute en orden, y no se
saltee ninguno. **CI/CD** automatiza exactamente esa secuencia.

## Qué es CI/CD

- **Integración continua** (CI): cada cambio se integra y se construye automáticamente.
  En este taller, eso es el build de CodeBuild disparándose con cada commit.
- **Entrega continua** (CD): cada cambio que pasa el build avanza automáticamente hacia
  el despliegue, con las verificaciones y aprobaciones que el equipo defina.

La idea central no es la velocidad por sí misma, sino la **reproducibilidad**: el camino
del código a producción es siempre el mismo, descrito una vez y ejecutado igual cada vez.
El mismo principio que vio en git (`push` en lugar de subir archivos) y en CloudFormation
(lanzar un template en lugar de clicar recursos), ahora aplicado al flujo entero.

## El pipeline

Un **pipeline** es la descripción de ese flujo automatizado. **AWS CodePipeline**
modela el camino como una secuencia de **etapas** (*stages*), y cada etapa contiene una
o más **acciones**.

:::inline-slide light
## Anatomía de un pipeline

```
Source        Build          Deploy
(CodeCommit) → (CodeBuild) → (ECS)
```

- **Stage** — una fase del flujo (Source, Build, Deploy).
- **Action** — un paso dentro de una etapa.
- **Artifact** — lo que una etapa produce y pasa a la siguiente.
- **Transition** — el paso de una etapa a la siguiente.
:::

- Una **etapa** agrupa un paso del flujo: obtener el código, construir, desplegar.
- Una **acción** es una tarea concreta dentro de una etapa (por ejemplo, "ejecutar este
  proyecto de CodeBuild").
- Un **artefacto** es lo que una etapa entrega a la siguiente: la etapa Source entrega el
  código, la etapa Build entrega la imagen.
- Una **transición** conecta una etapa con la siguiente; puede ser automática o estar
  detenida a la espera de una aprobación.

## Leer un pipeline existente

Antes de crear uno, conviene saber leerlo —en un equipo real, lo habitual es heredar
pipelines, no empezarlos de cero. La vista de CodePipeline muestra las etapas de
izquierda a derecha, cada una con el estado de su última ejecución: en curso, exitosa, o
fallida. Al leer un pipeline ajeno, las preguntas útiles son: ¿qué dispara la primera
etapa?, ¿qué produce cada etapa?, ¿dónde hay aprobaciones manuales?, y ¿en qué etapa
falla cuando falla?

:::slide
## Leer un pipeline

1. ¿Qué **dispara** la primera etapa?
2. ¿Qué **artefacto** produce cada etapa?
3. ¿Dónde hay **aprobaciones** manuales?
4. ¿En qué **etapa** falla?
:::

En la próxima sesión construirá un pipeline que automatiza el flujo que hoy hace a mano:
Source desde CodeCommit, Build con su proyecto de CodeBuild, y Deploy hacia ECS, con una
aprobación manual antes de desplegar.
