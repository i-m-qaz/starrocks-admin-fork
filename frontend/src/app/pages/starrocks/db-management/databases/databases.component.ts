import { Component, OnInit, ViewChild, TemplateRef } from '@angular/core';
import { ApiService } from '../../../../@core/data/api.service';
import { NbToastrService, NbDialogService, NbDialogRef } from '@nebular/theme';
import { LocalDataSource } from 'ng2-smart-table';

@Component({
  selector: 'ngx-databases',
  templateUrl: './databases.component.html',
  styleUrls: ['./databases.component.scss']
})
export class DatabasesComponent implements OnInit {

  @ViewChild('createDialog') createDialog!: TemplateRef<any>;
  @ViewChild('editDialog') editDialog!: TemplateRef<any>;

  clusterName = '当前集群';
  loading = false;
  creating = false;
  updating = false;

  source: LocalDataSource = new LocalDataSource();
  currentDatabase: any = null;
  newDatabase = {
    name: '',
    comment: ''
  };

  private dialogRef!: NbDialogRef<any>;

  settings = {
    actions: {
      add: false,
      edit: true,
      delete: true,
      position: 'right',
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
      name: {
        title: '数据库名称',
        type: 'string',
        editable: false,
      },
      comment: {
        title: '备注',
        type: 'string',
        editable: false,
      },
      tables_count: {
        title: '表数量',
        type: 'number',
        editable: false,
      },
      create_time: {
        title: '创建时间',
        type: 'string',
        editable: false,
        valuePrepareFunction: (cell: any) => {
          return cell ? new Date(cell).toLocaleString() : '-';
        }
      }
    }
  };

  constructor(
    private apiService: ApiService,
    private toastrService: NbToastrService,
    private dialogService: NbDialogService
  ) { }

  ngOnInit(): void {
    this.loadDatabases();
  }

  loadDatabases() {
    this.loading = true;
    this.apiService.get('/clusters/databases').subscribe(
      (data: any[]) => {
        this.source.load(data);
        this.loading = false;
      },
      error => {
        this.toastrService.danger('加载数据库失败', '错误');
        this.loading = false;
      }
    );
  }

  openCreateDialog() {
    this.newDatabase = { name: '', comment: '' };
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

  createDatabase() {
    if (!this.newDatabase.name.trim()) {
      this.toastrService.warning('请输入数据库名称', '警告');
      return;
    }

    this.creating = true;
    this.apiService.post('/clusters/databases', this.newDatabase).subscribe(
      (data: any) => {
        this.source.append(data);
        this.toastrService.success('数据库创建成功', '成功');
        this.closeCreateDialog();
        this.creating = false;
      },
      error => {
        this.toastrService.danger('创建数据库失败', '错误');
        this.creating = false;
      }
    );
  }

  onEdit(event: any) {
    this.currentDatabase = { ...event.data };
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
    this.currentDatabase = null;
  }

  updateDatabase() {
    if (!this.currentDatabase) return;

    this.updating = true;
    this.apiService.put(`/clusters/databases/${this.currentDatabase.name}`, {
      comment: this.currentDatabase.comment
    }).subscribe(
      (data: any) => {
        this.source.update(this.currentDatabase, data);
        this.toastrService.success('数据库更新成功', '成功');
        this.closeEditDialog();
        this.updating = false;
      },
      error => {
        this.toastrService.danger('更新数据库失败', '错误');
        this.updating = false;
      }
    );
  }

  onDelete(event: any) {
    const database = event.data;
    if (confirm(`确定要删除数据库 ${database.name} 吗？此操作不可恢复。`)) {
      this.apiService.delete(`/clusters/databases/${database.name}`).subscribe(
        () => {
          this.source.remove(database);
          this.toastrService.success('数据库删除成功', '成功');
        },
        error => {
          this.toastrService.danger('删除数据库失败', '错误');
        }
      );
    }
  }

}
