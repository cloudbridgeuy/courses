+++
title = "Introducción"
+++

:::slide
## ¿Qué es AWS DevOps?

Una combinación de prácticas culturales, servicios y herramientas que aumentan la
capacidad de entregar software a alta velocidad.
:::

:::slide
## Pipeline de entrega continua

```
Código → Build → Test → Deploy → Monitor
```
:::

## La narrativa del taller

Durante cuatro semanas usted desplegará y operará una aplicación web real de principio a fin,
sobre infraestructura de AWS. No se trata de ejercicios aislados: cada sesión avanza un paso
concreto de la misma historia. Al terminar la Semana 4, habrá recorrido el flujo completo
de un equipo moderno de desarrollo:

**CodeCommit → CodeBuild → ECR → ECS → CloudWatch**

El repositorio de código, la imagen Docker, el clúster de contenedores, y el monitoreo
—todo en AWS, todo creado por usted desde la consola, todo conectado entre sí.

## Qué construye la Semana 1

La primera semana establece los cimientos que el resto del taller supone conocidos. Al
terminar la sesión del viernes tendrá:

- Su **repositorio de código** en CodeCommit, con el código de la aplicación ya cargado.
- Un **pipeline de integración continua** en CodeBuild que, cada vez que lo ejecuta,
  compila la imagen Docker y la publica en Amazon ECR.
- La **aplicación en línea**: accesible desde el navegador a través de un Application
  Load Balancer, desplegada con una plantilla de CloudFormation provista por el instructor,
  sobre ECS/Fargate, conectada a una tabla de DynamoDB.

La plantilla de CloudFormation se usa esta semana como una caja negra: usted la lanza y
obtiene un ambiente funcional en minutos. Cómo está construida por dentro es el tema de
la Semana 2.

## El mecanismo de recuperación

Desde el primer día se practica destruir el ambiente completo y recrearlo desde cero.
Esto no es un ejercicio de destrucción: es el seguro del taller. Si algo sale mal en
cualquier sesión posterior —una configuración equivocada, un recurso corrompido— usted
borra el stack de CloudFormation, espera unos minutos, lo vuelve a lanzar con los mismos
parámetros, y queda en el mismo punto de partida. El costo de un error se reduce a
minutos de espera.

## Cómo usar esta guía

Cada sección combina teoría breve con práctica guiada paso a paso. El esquema es
siempre el mismo:

1. **Teoría** (10–15 min): el instructor presenta el problema del día y el servicio de
   AWS que lo resuelve.
2. **Práctica guiada**: siga los pasos numerados en la guía. Intente cada ejercicio por
   su cuenta antes de revelar la solución.
3. **Solución oculta**: si se atasca, pulse el botón **Ver solución** bajo cada ejercicio.
   Aparecerán los clics exactos. Al final de cada sesión todos los participantes quedan
   en el mismo punto.
4. **Preguntas puente**: al terminar la sesión del miércoles (presencial), aparecen dos
   o tres preguntas que conectan lo construido con lo que viene el viernes (remota).
   Piénselas antes de la siguiente sesión.

## Requisitos previos

Antes de comenzar, confirme que cuenta con lo siguiente:

- **Cuenta AWS** con permisos para CodeCommit, CodeBuild, ECR, CloudFormation, ECS y
  Fargate, y DynamoDB. Si no está seguro, consulte con el instructor.
- **Navegador web** actualizado (Chrome o Firefox recomendados).
- **Archivo `.zip` con el código de la aplicación**, provisto por el instructor al inicio
  del taller. Guárdelo en un lugar de fácil acceso.
- **Plantilla de CloudFormation** `taller-semana1.yaml`, también provista por el
  instructor. La usará en la sección 4.

## Nota sobre CodeCommit

En julio de 2024, AWS cerró el alta de CodeCommit para nuevas cuentas. Si su cuenta
fue creada después de esa fecha, es posible que no tenga acceso al servicio. En ese
caso, el instructor le indicará la alternativa que se usará para este taller. Todos los
demás servicios (CodeBuild, ECR, CloudFormation, ECS) no tienen esta restricción.
