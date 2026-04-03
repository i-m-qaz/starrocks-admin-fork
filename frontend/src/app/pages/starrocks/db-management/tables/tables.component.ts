import { Component, OnInit, ViewChild, TemplateRef } from '@angular/core';
import { ApiService } from '../../../../@core/data/api.service';
import { NbToastrService, NbDialogService, NbDialogRef } from '@nebular/theme';
import { LocalDataSource } from 'ng2-smart-table';
import { ClusterContextService } from '../../../../@core/data/cluster-context.service';
import { Cluster } from '../../../../@core/data/cluster.service';

@Component({
  selector: 'ngx-tables',
  templateUrl: './tables.component.html',
  styleUrls: ['./tables.component.scss']
})
export class TablesComponent implements OnInit {

  @ViewChild('createDialog') createDialog!: TemplateRef<any>;
  @ViewChild('editDialog') editDialog!: TemplateRef<any>;
  @ViewChild('actionDialog') actionDialog!: TemplateRef<any>;

  activeCluster: Cluster | null = null;
  loading = false;
  creating = false;
  updating = false;
  executingAction = false;

  databases: any[] = [];
  source: LocalDataSource = new LocalDataSource();
  currentTable: any = null;
  selectedTable: any = null;
  actionType: string | null = null;

  newTable = {
    database: '',
    name: '',
    table_type: 'duplicate',
    columns: [
      { name: 'id', data_type: 'BIGINT', is_nullable: false, is_key: true, comment: '', default_value: '', aggregate_type: '' }
    ],
    partition_info: {
      partition_type: 'range',
      partition_key: '',
      partitions: []
    },
    bucket_info: {
      bucket_type: 'hash',
      bucket_keys: '',
      bucket_count: 10
    },
    comment: '',
    properties: {}
  };

  tableTypes = [
    { value: 'duplicate', label: '明细表' },
    { value: 'aggregate', label: '聚合表' },
    { value: 'unique', label: '唯一表' },
    { value: 'primary', label: '主键表' },
    { value: 'update', label: '更新表' }
  ];

  dataTypes = [
    'BIGINT', 'INT', 'SMALLINT', 'TINYINT', 'BOOLEAN',
    'DECIMAL', 'DOUBLE', 'FLOAT',
    'VARCHAR', 'CHAR',
    'DATE', 'DATETIME', 'TIMESTAMP',
    'ARRAY', 'JSON'
  ];

  aggregateTypes = [
    'SUM', 'COUNT', 'MAX', 'MIN', 'AVG', 'BITMAP_UNION', 'HLL_UNION'
  ];

  partitionTypes = [
    { value: 'range', label: '范围分区' },
    { value: 'list', label: '列表分区' },
    { value: 'expression', label: '表达式分区' }
  ];

  bucketTypes = [
    { value: 'hash', label: '哈希分桶' },
    { value: 'range', label: '范围分桶' }
  ];

  private dialogRef!: NbDialogRef<any>;

  settings = {
    actions: {
      add: false,
      edit: true,
      delete: true,
      position: 'right',
      custom: [
        {
          name: 'truncate',
          title: '<i class="nb-trash"></i>',
        },
        {
          name: 'optimize',
          title: '<i class="nb-loop"></i>',
        }
      ]
    },
    edit: {
      editButtonContent: '<i class="nb-edit"></i>',
      saveButtonContent: '<i class="nb-checkmark"></i>',
      cancelButtonContent: '<i class="nb-close"></i>',
      confirmSave: false,
    },
    delete: {
      deleteButtonContent: '<i class="nb-trash"></i>',
      confirmDelete: true,
    },
    columns: {
      database: {
        title: '数据库',
        type: 'string',
        editable: false,
      },
      name: {
        title: '表名称',
        type: 'string',
        editable: false,
      },
      table_type: {
        title: '表类型',
        type: 'string',
        editable: false,
        valuePrepareFunction: (cell: any) => {
          const type = this.tableTypes.find(t => t.value === cell);
          return type ? type.label : cell;
        }
      },
      engine: {
        title: '引擎',
        type: 'string',
        editable: false,
      },
      rows: {
        title: '行数',
        type: 'number',
        editable: false,
        valuePrepareFunction: (cell: any) => {
          return cell || 0;
        }
      },
      comment: {
        title: '备注',
        type: 'string',
        editable: false,
      }
    }
  };

  constructor(
    private apiService: ApiService,
    private toastrService: NbToastrService,
    private dialogService: NbDialogService,
    private clusterContextService: ClusterContextService
  ) { }

  ngOnInit(): void {
    // Subscribe to active cluster changes
    this.clusterContextService.activeCluster$
      .subscribe(cluster => {
        this.activeCluster = cluster;
      });

    this.loadDatabases();
    this.loadTables();
  }

  loadDatabases() {
    this.apiService.get('/clusters/databases').subscribe(
      (data: any[]) => {
        this.databases = data;
        if (data.length > 0) {
          this.newTable.database = data[0].name;
        }
      },
      error => {
        this.toastrService.danger('加载数据库失败', '错误');
      }
    );
  }

  loadTables() {
    this.loading = true;
    this.apiService.get('/clusters/tables').subscribe(
      (data: any[]) => {
        this.source.load(data);
        this.loading = false;
      },
      error => {
        this.toastrService.danger('加载表失败', '错误');
        this.loading = false;
      }
    );
  }

  openCreateDialog() {
    this.resetNewTable();
    this.dialogRef = this.dialogService.open(this.createDialog, {
      context: {},
      hasBackdrop: true,
      closeOnBackdropClick: false,
    });
  }

  closeCreateDialog() {
    if (this.dialogRef) {
      this.dialogRef.close();
    }
  }

  resetNewTable() {
    this.newTable = {
      database: this.databases.length > 0 ? this.databases[0].name : '',
      name: '',
      table_type: 'duplicate',
      columns: [
        { name: 'id', data_type: 'BIGINT', is_nullable: false, is_key: true, comment: '', default_value: '', aggregate_type: '' }
      ],
      partition_info: {
        partition_type: 'range',
        partition_key: '',
        partitions: []
      },
      bucket_info: {
        bucket_type: 'hash',
        bucket_keys: '',
        bucket_count: 10
      },
      comment: '',
      properties: {}
    };
  }

  addColumn() {
    this.newTable.columns.push({
      name: '',
      data_type: 'VARCHAR',
      is_nullable: true,
      is_key: false,
      comment: '',
      default_value: '',
      aggregate_type: ''
    });
  }

  removeColumn(index: number) {
    if (this.newTable.columns.length > 1) {
      this.newTable.columns.splice(index, 1);
    }
  }

  addPartition() {
    this.newTable.partition_info.partitions.push({
      name: '',
      values: ''
    });
  }

  removePartition(index: number) {
    this.newTable.partition_info.partitions.splice(index, 1);
  }

  createTable() {
    if (!this.newTable.name.trim() || !this.newTable.database || this.newTable.columns.length === 0) {
      this.toastrService.warning('请填写必要的表信息', '警告');
      return;
    }

    this.creating = true;
    this.apiService.post('/clusters/tables', this.newTable).subscribe(
      (data: any) => {
        this.source.append(data);
        this.toastrService.success('表创建成功', '成功');
        this.closeCreateDialog();
        this.creating = false;
      },
      error => {
        this.toastrService.danger('创建表失败', '错误');
        this.creating = false;
      }
    );
  }

  onEdit(event: any) {
    this.currentTable = { ...event.data };
    this.dialogRef = this.dialogService.open(this.editDialog, {
      context: {},
      hasBackdrop: true,
      closeOnBackdropClick: false,
    });
  }

  closeEditDialog() {
    if (this.dialogRef) {
      this.dialogRef.close();
    }
    this.currentTable = null;
  }

  updateTable() {
    if (!this.currentTable) return;

    this.updating = true;
    this.apiService.put(`/clusters/tables/${this.currentTable.database}/${this.currentTable.name}`, {
      comment: this.currentTable.comment
    }).subscribe(
      (data: any) => {
        this.source.update(this.currentTable, data);
        this.toastrService.success('表更新成功', '成功');
        this.closeEditDialog();
        this.updating = false;
      },
      error => {
        this.toastrService.danger('更新表失败', '错误');
        this.updating = false;
      }
    );
  }

  onDelete(event: any) {
    const table = event.data;
    if (confirm(`确定要删除表 ${table.database}.${table.name} 吗？此操作不可恢复。`)) {
      this.apiService.delete(`/clusters/tables/${table.database}/${table.name}`).subscribe(
        () => {
          this.source.remove(table);
          this.toastrService.success('表删除成功', '成功');
        },
        error => {
          this.toastrService.danger('删除表失败', '错误');
        }
      );
    }
  }

  openActionDialog(table: any, action: string) {
    this.selectedTable = table;
    this.actionType = action;
    this.dialogRef = this.dialogService.open(this.actionDialog, {
      context: {},
      hasBackdrop: true,
      closeOnBackdropClick: false,
    });
  }

  closeActionDialog() {
    if (this.dialogRef) {
      this.dialogRef.close();
    }
    this.selectedTable = null;
    this.actionType = null;
  }

  executeTableAction() {
    if (!this.selectedTable || !this.actionType) return;

    this.executingAction = true;
    this.apiService.post(`/clusters/tables/${this.selectedTable.database}/${this.selectedTable.name}/action`, {
      action: this.actionType
    }).subscribe(
      () => {
        this.toastrService.success(`${this.getActionTitle(this.actionType)} 操作成功`, '成功');
        this.closeActionDialog();
        this.executingAction = false;
      },
      error => {
        this.toastrService.danger(`${this.getActionTitle(this.actionType)} 操作失败`, '错误');
        this.executingAction = false;
      }
    );
  }

  getActionTitle(action: string): string {
    const actionMap: { [key: string]: string } = {
      truncate: '截断表',
      optimize: '优化表'
    };
    return actionMap[action] || action;
  }

  getActionDescription(action: string): string {
    const actionMap: { [key: string]: string } = {
      truncate: '截断',
      optimize: '优化'
    };
    return actionMap[action] || action;
  }

  getActionStatus(action: string): string {
    const actionMap: { [key: string]: string } = {
      truncate: 'warning',
      optimize: 'info'
    };
    return actionMap[action] || 'primary';
  }

  getActionIcon(action: string): string {
    const actionMap: { [key: string]: string } = {
      truncate: 'trash-outline',
      optimize: 'loop-outline'
    };
    return actionMap[action] || 'checkmark-outline';
  }

  onCustomAction(event: any) {
    this.openActionDialog(event.data, event.action);
  }

}
