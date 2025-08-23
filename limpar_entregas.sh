#!/bin/bash

# Diretórios de entregas
ENTREGAS_SHU="entregas/shu"
ENTREGAS_WEI="entregas/wei"
ENTREGAS_WU="entregas/wu"

# Arquivos de registro
REGISTRO_SHU="registro_shu.json"
REGISTRO_WEI="registro_wei.json"
REGISTRO_WU="registro_wu.json"

echo "Iniciando a limpeza do histórico de entregas..."

# Limpa o diretório de entregas do time Shu
if [ -d "$ENTREGAS_SHU" ]; then
    echo "Limpando o diretório $ENTREGAS_SHU..."
    rm -f $ENTREGAS_SHU/*
else
    echo "Diretório $ENTREGAS_SHU não encontrado."
fi

# Limpa o diretório de entregas do time Wei
if [ -d "$ENTREGAS_WEI" ]; then
    echo "Limpando o diretório $ENTREGAS_WEI..."
    rm -f $ENTREGAS_WEI/*
else
    echo "Diretório $ENTREGAS_WEI não encontrado."
fi

# Limpa o diretório de entregas do time Wu
if [ -d "$ENTREGAS_WU" ]; then
    echo "Limpando o diretório $ENTREGAS_WU..."
    rm -f $ENTREGAS_WU/*
else
    echo "Diretório $ENTREGAS_WU não encontrado."
fi

# Limpa o conteúdo do arquivo de registro do time Shu
if [ -f "$REGISTRO_SHU" ]; then
    echo "Limpando o conteúdo de $REGISTRO_SHU..."
    echo "[]" > $REGISTRO_SHU
else
    echo "Arquivo $REGISTRO_SHU não encontrado, criando um novo."
    echo "[]" > $REGISTRO_SHU
fi

# Limpa o conteúdo do arquivo de registro do time Wei
if [ -f "$REGISTRO_WEI" ]; then
    echo "Limpando o conteúdo de $REGISTRO_WEI..."
    echo "[]" > $REGISTRO_WEI
else
    echo "Arquivo $REGISTRO_WEI não encontrado, criando um novo."
    echo "[]" > $REGISTRO_WEI
fi

# Limpa o conteúdo do arquivo de registro do time Wu
if [ -f "$REGISTRO_WU" ]; then
    echo "Limpando o conteúdo de $REGISTRO_WU..."
    echo "[]" > $REGISTRO_WU
else
    echo "Arquivo $REGISTRO_WU não encontrado, criando um novo."
    echo "[]" > $REGISTRO_WU
fi

echo "Limpeza do histórico de entregas concluída com sucesso!"
